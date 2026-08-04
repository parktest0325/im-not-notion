//! Remote filesystem ops for the file explorer.
//! Separate from `file_service` (which is sidebar/Hugo-specific).

use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::services::ssh_service::{execute_ssh_command_checked, get_channel_session, get_sftp_session};

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<u64>,    // unix epoch seconds
}

pub fn list_remote_dir(path: &str) -> Result<Vec<FsEntry>> {
    let sftp = get_sftp_session()?;
    let dir_path = if path.is_empty() { "." } else { path };
    let entries = sftp.readdir(Path::new(dir_path))?;
    let mut out = Vec::new();
    for (p, stat) in entries {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name.is_empty() { continue; }
        out.push(FsEntry {
            name,
            is_dir: stat.is_dir(),
            size: if stat.is_dir() { 0 } else { stat.size.unwrap_or(0) },
            modified: stat.mtime,
        });
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(out)
}

/// 원격 조작 경로 검증: 절대경로 + `..` 없음 + 루트("/")가 아님.
/// 탐색기 UI 버그가 `rm -rf /` 같은 사고로 이어지지 않게 하는 최후 방어선.
fn validate_remote_path(path: &str) -> Result<()> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || !path.starts_with('/') {
        anyhow::bail!("Refusing unsafe remote path: {:?}", path);
    }
    if path.split('/').any(|c| c == "..") {
        anyhow::bail!("Refusing remote path containing '..': {:?}", path);
    }
    Ok(())
}

pub fn delete_remote(paths: &[String]) -> Result<()> {
    if paths.is_empty() { return Ok(()); }
    for p in paths {
        validate_remote_path(p)?;
    }
    let args = paths.iter().map(|p| shq(p)).collect::<Vec<_>>().join(" ");
    let mut ch = get_channel_session()?;
    execute_ssh_command_checked(&mut ch, &format!("rm -rf {}", args))?;
    Ok(())
}

pub fn move_remote(src: &str, dst: &str) -> Result<()> {
    validate_remote_path(src)?;
    validate_remote_path(dst)?;
    let mut ch = get_channel_session()?;
    execute_ssh_command_checked(&mut ch, &format!("mv {} {}", shq(src), shq(dst)))?;
    Ok(())
}

pub fn mkdir_remote(path: &str) -> Result<()> {
    validate_remote_path(path)?;
    let mut ch = get_channel_session()?;
    execute_ssh_command_checked(&mut ch, &format!("mkdir -p {}", shq(path)))?;
    Ok(())
}

pub fn get_home_dir_remote() -> Result<String> {
    crate::services::ssh_service::get_server_home_path()
}

use crate::utils::shell::quote as shq;
