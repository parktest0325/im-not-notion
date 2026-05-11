//! tar-bundled upload/download with progress events.
//!
//! Pipeline:
//!   upload   : local files → local tar.gz → SFTP → SSH `tar xzf` → cleanup
//!   download : SSH `tar czf` → SFTP fetch → local extract → cleanup

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use tar::Builder;
use tauri::Emitter;
use walkdir::WalkDir;

use crate::services::ssh_service::{execute_ssh_command, get_channel_session, get_sftp_session};

/* ===== Types ===== */

#[typeshare::typeshare]
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictPolicy {
    Overwrite,
    Skip,
    Rename,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ConflictItem {
    pub name: String,        // basename
    pub is_dir: bool,
    pub size: u64,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct TransferProgress {
    pub id: String,
    pub phase: String,           // "packing" | "uploading" | "extracting" | "downloading" | "cleanup" | "done" | "error"
    pub current_bytes: u64,
    pub total_bytes: u64,
    pub files_done: u32,
    pub files_total: u32,
    pub current_file: String,
    pub error: Option<String>,
}

/* ===== Helpers ===== */

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

fn emit_progress(app: &tauri::AppHandle, p: &TransferProgress) {
    let _ = app.emit("transfer:progress", p);
}

/// Quote a path for safe inclusion in a single SSH command line.
fn shq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/* ===== Conflict check ===== */

pub fn check_upload_conflicts(
    local_paths: &[String],
    remote_dir: &str,
) -> Result<Vec<ConflictItem>> {
    let sftp = get_sftp_session()?;
    let mut conflicts = Vec::new();
    for p in local_paths {
        let Some(name) = Path::new(p).file_name().and_then(|n| n.to_str()) else { continue };
        let remote = format!("{}/{}", remote_dir.trim_end_matches('/'), name);
        if let Ok(stat) = sftp.stat(Path::new(&remote)) {
            conflicts.push(ConflictItem {
                name: name.to_string(),
                is_dir: stat.is_dir(),
                size: stat.size.unwrap_or(0),
            });
        }
    }
    Ok(conflicts)
}

pub fn check_download_conflicts(
    remote_paths: &[String],
    local_dir: &str,
) -> Result<Vec<ConflictItem>> {
    let mut conflicts = Vec::new();
    for p in remote_paths {
        let Some(name) = Path::new(p).file_name().and_then(|n| n.to_str()) else { continue };
        let local = PathBuf::from(local_dir).join(name);
        if let Ok(meta) = std::fs::metadata(&local) {
            conflicts.push(ConflictItem {
                name: name.to_string(),
                is_dir: meta.is_dir(),
                size: meta.len(),
            });
        }
    }
    Ok(conflicts)
}

/* ===== Policy: filter/rename top-level basenames ===== */

/// Returns the list of `(path, archive_name)` pairs after applying the policy.
/// `existing_names` is the set of conflicting basenames detected earlier.
fn apply_policy(
    paths: &[String],
    existing: &[String],
    policy: ConflictPolicy,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for p in paths {
        let Some(name) = Path::new(p).file_name().and_then(|n| n.to_str()) else { continue };
        let conflicts = existing.iter().any(|n| n == name);
        if conflicts {
            match policy {
                ConflictPolicy::Overwrite => out.push((p.clone(), name.to_string())),
                ConflictPolicy::Skip => { /* drop */ }
                ConflictPolicy::Rename => out.push((p.clone(), rename_basename(name, existing))),
            }
        } else {
            out.push((p.clone(), name.to_string()));
        }
    }
    out
}

fn rename_basename(name: &str, existing: &[String]) -> String {
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = path.extension().and_then(|e| e.to_str());
    let mut i = 1;
    loop {
        let candidate = match ext {
            Some(e) => format!("{}_{}.{}", stem, i, e),
            None => format!("{}_{}", stem, i),
        };
        if !existing.iter().any(|n| n == &candidate) {
            return candidate;
        }
        i += 1;
        if i > 1000 {
            return format!("{}_{}", stem, new_id());
        }
    }
}

/* ===== Upload ===== */

pub fn upload_to_remote(
    local_paths: Vec<String>,
    remote_dir: String,
    policy: ConflictPolicy,
    app: tauri::AppHandle,
) -> Result<String> {
    let id = new_id();
    match upload_inner(&id, local_paths, remote_dir, policy, &app) {
        Ok(()) => Ok(id),
        Err(e) => {
            emit_progress(&app, &TransferProgress {
                id: id.clone(), phase: "error".into(),
                current_bytes: 0, total_bytes: 0,
                files_done: 0, files_total: 0,
                current_file: String::new(),
                error: Some(format!("{:#}", e)),
            });
            Err(e)
        }
    }
}

fn upload_inner(
    id: &str,
    local_paths: Vec<String>,
    remote_dir: String,
    policy: ConflictPolicy,
    app: &tauri::AppHandle,
) -> Result<()> {
    // Conflict check & policy application
    let conflicts = check_upload_conflicts(&local_paths, &remote_dir).unwrap_or_default();
    let existing_names: Vec<String> = conflicts.iter().map(|c| c.name.clone()).collect();
    let entries = apply_policy(&local_paths, &existing_names, policy);

    if entries.is_empty() {
        emit_progress(app, &TransferProgress {
            id: id.into(),
            phase: "done".into(),
            current_bytes: 0, total_bytes: 0,
            files_done: 0, files_total: 0,
            current_file: String::new(), error: None,
        });
        return Ok(());
    }

    // ── Phase 1: Pack ──
    let temp_tar = std::env::temp_dir().join(format!("inn-transfer-{}.tgz", id));
    let (files_total, total_bytes) = count_local(&entries);

    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "packing".into(),
        current_bytes: 0, total_bytes,
        files_done: 0, files_total,
        current_file: String::new(), error: None,
    });

    let mut files_done = 0u32;
    let mut bytes_done = 0u64;
    build_tar(&entries, &temp_tar, |file, bytes| {
        files_done += 1;
        bytes_done += bytes;
        emit_progress(app, &TransferProgress {
            id: id.into(), phase: "packing".into(),
            current_bytes: bytes_done, total_bytes,
            files_done, files_total,
            current_file: file.to_string(), error: None,
        });
    })?;

    // ── Phase 2: SFTP upload ──
    let remote_tar = format!("/tmp/inn-transfer-{}.tgz", id);
    let tar_size = std::fs::metadata(&temp_tar)?.len();
    let short_id = &id[..8.min(id.len())];
    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "uploading".into(),
        current_bytes: 0, total_bytes: tar_size,
        files_done: files_total, files_total,
        current_file: format!("{}.tgz", short_id),
        error: None,
    });
    sftp_upload(&temp_tar, &remote_tar, |sent| {
        emit_progress(app, &TransferProgress {
            id: id.into(), phase: "uploading".into(),
            current_bytes: sent, total_bytes: tar_size,
            files_done: files_total, files_total,
            current_file: format!("{}.tgz", short_id),
            error: None,
        });
    })?;

    // ── Phase 3: Remote extract ──
    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "extracting".into(),
        current_bytes: tar_size, total_bytes: tar_size,
        files_done: files_total, files_total,
        current_file: remote_dir.clone(), error: None,
    });
    let mut ch = get_channel_session()?;
    let mkdir_cmd = format!("mkdir -p {}", shq(&remote_dir));
    execute_ssh_command(&mut ch, &mkdir_cmd)
        .with_context(|| format!("mkdir failed: {}", mkdir_cmd))?;
    let mut ch = get_channel_session()?;
    let extract_cmd = format!(
        "tar xzf {} -C {}",
        shq(&remote_tar),
        shq(&remote_dir),
    );
    execute_ssh_command(&mut ch, &extract_cmd)
        .with_context(|| format!("tar extract failed: {}", extract_cmd))?;

    // ── Phase 4: Cleanup ──
    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "cleanup".into(),
        current_bytes: tar_size, total_bytes: tar_size,
        files_done: files_total, files_total,
        current_file: String::new(), error: None,
    });
    let _ = std::fs::remove_file(&temp_tar);
    let mut ch = get_channel_session()?;
    let _ = execute_ssh_command(&mut ch, &format!("rm -f {}", shq(&remote_tar)));

    // ── Done ──
    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "done".into(),
        current_bytes: tar_size, total_bytes: tar_size,
        files_done: files_total, files_total,
        current_file: String::new(), error: None,
    });

    Ok(())
}

/* ===== Download ===== */

pub fn download_to_local(
    remote_paths: Vec<String>,
    local_dir: String,
    policy: ConflictPolicy,
    app: tauri::AppHandle,
) -> Result<String> {
    let id = new_id();
    match download_inner(&id, remote_paths, local_dir, policy, &app) {
        Ok(()) => Ok(id),
        Err(e) => {
            emit_progress(&app, &TransferProgress {
                id: id.clone(), phase: "error".into(),
                current_bytes: 0, total_bytes: 0,
                files_done: 0, files_total: 0,
                current_file: String::new(),
                error: Some(format!("{:#}", e)),
            });
            Err(e)
        }
    }
}

fn download_inner(
    id: &str,
    remote_paths: Vec<String>,
    local_dir: String,
    policy: ConflictPolicy,
    app: &tauri::AppHandle,
) -> Result<()> {
    // Conflict check & policy
    let conflicts = check_download_conflicts(&remote_paths, &local_dir).unwrap_or_default();
    let existing_names: Vec<String> = conflicts.iter().map(|c| c.name.clone()).collect();

    let entries = apply_policy(&remote_paths, &existing_names, policy);
    if entries.is_empty() {
        emit_progress(app, &TransferProgress {
            id: id.into(), phase: "done".into(),
            current_bytes: 0, total_bytes: 0, files_done: 0, files_total: 0,
            current_file: String::new(), error: None,
        });
        return Ok(());
    }

    let original_paths: Vec<String> = entries.iter().map(|(p, _)| p.clone()).collect();
    let parent = common_parent(&original_paths)
        .context("Selected items must share the same parent folder")?;
    let names: Vec<String> = original_paths
        .iter()
        .filter_map(|p| Path::new(p).file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();
    if names.is_empty() {
        return Err(anyhow::anyhow!("No files selected"));
    }

    // ── Phase 1: Pack on remote ──
    let remote_tar = format!("/tmp/inn-transfer-{}.tgz", id);
    let names_joined = names.iter().map(|n| shq(n)).collect::<Vec<_>>().join(" ");
    // Use `tar -C` instead of `cd && tar` — more reliable across shells.
    let pack_cmd = format!(
        "tar czf {} -C {} {}",
        shq(&remote_tar), shq(&parent), names_joined
    );
    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "packing".into(),
        current_bytes: 0, total_bytes: 0,
        files_done: 0, files_total: names.len() as u32,
        current_file: format!("{} (packing on server...)", parent),
        error: None,
    });
    let mut ch = get_channel_session()?;
    execute_ssh_command(&mut ch, &pack_cmd)
        .with_context(|| format!("tar pack failed: {}", pack_cmd))?;

    // ── Phase 2: SFTP download ──
    let tar_size = {
        let sftp = get_sftp_session()?;
        let stat = sftp.stat(Path::new(&remote_tar))?;
        stat.size.unwrap_or(0)
    };

    let temp_tar = std::env::temp_dir().join(format!("inn-transfer-{}.tgz", id));
    let short_id = &id[..8.min(id.len())];
    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "downloading".into(),
        current_bytes: 0, total_bytes: tar_size,
        files_done: 0, files_total: names.len() as u32,
        current_file: format!("{}.tgz", short_id),
        error: None,
    });
    sftp_download(&remote_tar, &temp_tar, |recv| {
        emit_progress(app, &TransferProgress {
            id: id.into(), phase: "downloading".into(),
            current_bytes: recv, total_bytes: tar_size,
            files_done: 0, files_total: names.len() as u32,
            current_file: format!("{}.tgz", short_id),
            error: None,
        });
    })?;

    // ── Phase 3: Local extract ──
    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "extracting".into(),
        current_bytes: tar_size, total_bytes: tar_size,
        files_done: 0, files_total: names.len() as u32,
        current_file: local_dir.clone(), error: None,
    });
    std::fs::create_dir_all(&local_dir)?;
    extract_local_tar(&temp_tar, Path::new(&local_dir))?;

    // For Rename policy, post-rename files on local side
    if matches!(policy, ConflictPolicy::Rename) {
        for (orig, archive_name) in &entries {
            let from = PathBuf::from(&local_dir)
                .join(Path::new(orig).file_name().unwrap_or_default());
            let to = PathBuf::from(&local_dir).join(archive_name);
            if from != to && from.exists() {
                let _ = std::fs::rename(&from, &to);
            }
        }
    }

    // ── Phase 4: Cleanup ──
    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "cleanup".into(),
        current_bytes: tar_size, total_bytes: tar_size,
        files_done: names.len() as u32, files_total: names.len() as u32,
        current_file: String::new(), error: None,
    });
    let _ = std::fs::remove_file(&temp_tar);
    let mut ch = get_channel_session()?;
    let _ = execute_ssh_command(&mut ch, &format!("rm -f {}", shq(&remote_tar)));

    emit_progress(app, &TransferProgress {
        id: id.into(), phase: "done".into(),
        current_bytes: tar_size, total_bytes: tar_size,
        files_done: names.len() as u32, files_total: names.len() as u32,
        current_file: String::new(), error: None,
    });

    Ok(())
}


fn common_parent(paths: &[String]) -> Option<String> {
    let parents: Vec<String> = paths
        .iter()
        .filter_map(|p| {
            Path::new(p).parent()
                .and_then(|pp| pp.to_str())
                .map(|s| if s.is_empty() { "/".to_string() } else { s.to_string() })
        })
        .collect();
    if parents.is_empty() { return None; }
    let first = parents[0].clone();
    if parents.iter().all(|p| p == &first) { Some(first) } else { None }
}

/* ===== Tar helpers ===== */

fn count_local(entries: &[(String, String)]) -> (u32, u64) {
    let mut files = 0u32;
    let mut bytes = 0u64;
    for (p, _) in entries {
        for e in WalkDir::new(p).into_iter().filter_map(|r| r.ok()) {
            if e.file_type().is_file() {
                files += 1;
                if let Ok(m) = e.metadata() {
                    bytes += m.len();
                }
            }
        }
    }
    (files, bytes)
}

fn build_tar(
    entries: &[(String, String)],
    output: &Path,
    mut on_file: impl FnMut(&str, u64),
) -> Result<()> {
    let file = File::create(output)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(enc);
    builder.follow_symlinks(false);

    for (path, archive_name) in entries {
        let p = Path::new(path);
        if p.is_file() {
            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            builder.append_path_with_name(p, archive_name)?;
            on_file(archive_name, size);
        } else if p.is_dir() {
            // Walk the dir, appending each entry under archive_name/...
            for entry in WalkDir::new(p).into_iter().filter_map(|r| r.ok()) {
                let abs = entry.path();
                let rel = abs.strip_prefix(p).unwrap_or(abs);
                let in_archive = PathBuf::from(archive_name).join(rel);
                if entry.file_type().is_dir() {
                    builder.append_dir(&in_archive, abs)?;
                } else if entry.file_type().is_file() {
                    let size = std::fs::metadata(abs).map(|m| m.len()).unwrap_or(0);
                    builder.append_path_with_name(abs, &in_archive)?;
                    on_file(&in_archive.to_string_lossy(), size);
                }
            }
        }
    }
    builder.finish()?;
    Ok(())
}

fn extract_local_tar(tar_path: &Path, target_dir: &Path) -> Result<()> {
    let file = File::open(tar_path)?;
    let dec = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(dec);
    archive.set_overwrite(true);
    archive.unpack(target_dir)?;
    Ok(())
}

/* ===== SFTP transfer with progress ===== */

fn sftp_upload(local: &Path, remote: &str, mut on_progress: impl FnMut(u64)) -> Result<()> {
    let total = std::fs::metadata(local)?.len();
    let sftp = get_sftp_session()?;
    let mut remote_file = sftp.create(Path::new(remote))?;
    let mut local_file = File::open(local)?;

    let mut buf = vec![0u8; 64 * 1024];
    let mut sent: u64 = 0;
    let mut last = Instant::now();
    on_progress(0);

    loop {
        let n = local_file.read(&mut buf)?;
        if n == 0 { break; }
        remote_file.write_all(&buf[..n])?;
        sent += n as u64;
        if last.elapsed() > Duration::from_millis(100) {
            on_progress(sent);
            last = Instant::now();
        }
    }
    on_progress(total);
    Ok(())
}

fn sftp_download(remote: &str, local: &Path, mut on_progress: impl FnMut(u64)) -> Result<()> {
    let sftp = get_sftp_session()?;
    let mut remote_file = sftp.open(Path::new(remote))?;
    let mut local_file = File::create(local)?;

    let mut buf = vec![0u8; 64 * 1024];
    let mut recv: u64 = 0;
    let mut last = Instant::now();
    on_progress(0);

    loop {
        let n = remote_file.read(&mut buf)?;
        if n == 0 { break; }
        local_file.write_all(&buf[..n])?;
        recv += n as u64;
        if last.elapsed() > Duration::from_millis(100) {
            on_progress(recv);
            last = Instant::now();
        }
    }
    on_progress(recv);
    Ok(())
}
