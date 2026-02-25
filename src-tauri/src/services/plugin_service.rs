use std::path::Path;
use std::io::prelude::*;
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use anyhow::{Result, Context, bail};
use crate::services::ssh_service::{get_channel_session, get_sftp_session, execute_ssh_command, get_server_home_path};
use crate::services::config_service::get_hugo_config;
use crate::services::file_service::{mkdir_recursive, rmrf_file};
use crate::types::plugin::*;

const PLUGIN_DIR: &str = "$HOME/.inn_plugins";

/// ~ 를 실제 홈 경로로 치환
fn resolve_plugin_dir() -> Result<String> {
    let home = get_server_home_path()?;
    Ok(format!("{}/.inn_plugins", home))
}

// ============================================================
// 조회
// ============================================================

/// 서버에 설치된 플러그인 목록 (enabled 상태 + 해시 포함)
fn discover_server_plugins() -> Result<Vec<(PluginManifest, bool, String)>> {
    let mut channel = get_channel_session()?;
    // plugin.json 내용 + .disabled 여부 + 디렉토리 해시를 한 번에 조회
    let cmd = format!(
        "for d in {0}/*/plugin.json; do \
            [ -f \"$d\" ] || continue; \
            dir=$(dirname \"$d\"); \
            name=$(basename \"$dir\"); \
            disabled=\"false\"; \
            [ -f \"$dir/.disabled\" ] && disabled=\"true\"; \
            hash=$(cd \"$dir\" && find . -type f ! -path '*/.*' | while IFS= read -r f; do printf '%s  %s\\n' \"$(tr -d '\\r' < \"$f\" | sha256sum | head -c 64)\" \"$f\"; done | sort | sha256sum | awk '{{print $1}}'); \
            echo \"---ENTRY---\"; \
            echo \"$disabled\"; \
            echo \"$hash\"; \
            cat \"$d\"; \
        done",
        PLUGIN_DIR
    );
    let output = execute_ssh_command(&mut channel, &cmd)?;

    let mut plugins = Vec::new();
    for chunk in output.split("---ENTRY---") {
        let trimmed = chunk.trim();
        if trimmed.is_empty() { continue; }

        // 첫 줄: disabled, 둘째 줄: hash, 나머지: plugin.json
        let mut lines = trimmed.splitn(3, '\n');
        let disabled_str = lines.next().unwrap_or("false").trim();
        let hash_str = lines.next().unwrap_or("").trim().to_string();
        let json_str = lines.next().unwrap_or("").trim();

        if json_str.is_empty() { continue; }
        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(json_str) {
            let enabled = disabled_str != "true";
            plugins.push((manifest, enabled, hash_str));
        }
    }
    Ok(plugins)
}

/// 로컬 플러그인 디렉토리에서 manifest 목록 읽기
fn discover_local_plugins(local_path: &str) -> Result<Vec<PluginManifest>> {
    let local = Path::new(local_path);
    if !local.is_dir() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();
    for entry in std::fs::read_dir(local)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() { continue; }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }

        let json_path = entry.path().join("plugin.json");
        if !json_path.exists() { continue; }

        if let Ok(data) = std::fs::read_to_string(&json_path) {
            if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&data) {
                plugins.push(manifest);
            }
        }
    }
    Ok(plugins)
}

/// 로컬 플러그인 디렉토리의 해시 계산 (pure Rust, \r 정규화)
/// 서버 해시와 동일한 로직: 각 파일의 sha256(\r 제거) + 상대경로 → 정렬 → 전체 sha256
fn compute_local_hash(plugin_dir: &Path) -> String {
    let mut entries: Vec<(String, String)> = Vec::new(); // (hash, relative_path)

    fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, String)>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') { continue; } // skip hidden
            if path.is_dir() {
                walk(&path, base, out);
            } else if path.is_file() {
                let Ok(raw) = std::fs::read(&path) else { continue };
                // Strip \r to normalize CRLF→LF (matches server tr -d '\r')
                let normalized: Vec<u8> = raw.into_iter().filter(|&b| b != b'\r').collect();
                let hash = format!("{:x}", Sha256::digest(&normalized));
                // Relative path with forward slashes (matches server `find .`)
                let rel = path.strip_prefix(base)
                    .map(|p| format!("./{}", p.to_string_lossy().replace('\\', "/")))
                    .unwrap_or_default();
                out.push((hash, rel));
            }
        }
    }

    walk(plugin_dir, plugin_dir, &mut entries);
    if entries.is_empty() { return String::new(); }

    // Format like sha256sum output: "hash  filename\n", sort, then hash the whole thing
    let mut lines: Vec<String> = entries.into_iter()
        .map(|(h, p)| format!("{}  {}", h, p))
        .collect();
    lines.sort();
    let combined = lines.join("\n") + "\n";
    format!("{:x}", Sha256::digest(combined.as_bytes()))
}

/// 로컬 + 서버 플러그인 병합 리스트
pub fn list_all_plugins(local_path: &str) -> Result<Vec<PluginInfo>> {
    let server_plugins = discover_server_plugins().unwrap_or_default();
    let local_plugins = discover_local_plugins(local_path).unwrap_or_default();

    // 서버 플러그인을 name → (manifest, enabled, hash) 맵으로
    let mut server_map: HashMap<String, (PluginManifest, bool, String)> = HashMap::new();
    for (manifest, enabled, hash) in server_plugins {
        server_map.insert(manifest.name.clone(), (manifest, enabled, hash));
    }

    let mut result: Vec<PluginInfo> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let has_local_path = !local_path.is_empty();

    // 로컬 플러그인 우선 순회
    for local_manifest in &local_plugins {
        seen.insert(local_manifest.name.clone());
        if let Some((_, enabled, server_hash)) = server_map.get(&local_manifest.name) {
            let local_hash = compute_local_hash(
                &Path::new(local_path).join(&local_manifest.name)
            );
            let synced = !local_hash.is_empty() && !server_hash.is_empty()
                && local_hash == *server_hash;
            result.push(PluginInfo {
                manifest: local_manifest.clone(),
                local: true,
                installed: true,
                enabled: *enabled,
                synced,
            });
        } else {
            result.push(PluginInfo {
                manifest: local_manifest.clone(),
                local: true,
                installed: false,
                enabled: false,
                synced: false,
            });
        }
    }

    // 서버에만 있는 플러그인
    for (name, (manifest, enabled, _)) in &server_map {
        if !seen.contains(name) {
            result.push(PluginInfo {
                manifest: manifest.clone(),
                local: false,
                installed: true,
                enabled: *enabled,
                synced: !has_local_path, // localPath 미설정이면 비교 불가 → true 취급
            });
        }
    }

    Ok(result)
}

// ============================================================
// Install / Uninstall
// ============================================================

/// 로컬 플러그인 하나를 서버에 설치 (tar 압축 → 단일 업로드 → 서버 해제)
pub fn install_plugin(local_path: &str, plugin_name: &str) -> Result<()> {
    let sftp = get_sftp_session()?;
    let remote_base = resolve_plugin_dir()?;

    let local_dir = Path::new(local_path).join(plugin_name);
    if !local_dir.is_dir() {
        anyhow::bail!("Plugin not found locally: {}", plugin_name);
    }

    // 1. 로컬에서 tar.gz 생성
    let tar_data = create_tar_gz(&local_dir)?;

    // 2. 서버에 tar.gz 업로드
    let remote_tar = format!("{}/{}.tar.gz", remote_base, plugin_name);
    mkdir_recursive(&sftp, Path::new(&remote_base))?;
    let mut remote_file = sftp.create(Path::new(&remote_tar))?;
    remote_file.write_all(&tar_data)?;

    // 3. 서버에서 기존 폴더 삭제 → 압축 해제 → tar.gz 삭제
    let remote_dir = format!("{}/{}", remote_base, plugin_name);
    let mut channel = get_channel_session()?;
    execute_ssh_command(
        &mut channel,
        &format!(
            "rm -rf '{}' && mkdir -p '{}' && tar -xzf '{}' -C '{}' && rm -f '{}' && find '{}' -type f \\( -name '*.py' -o -name '*.sh' -o -name '*.json' \\) -exec sed -i 's/\\r$//' {{}} +",
            remote_dir, remote_dir, remote_tar, remote_dir, remote_tar, remote_dir
        ),
    )?;

    // 4. 실행 권한 부여
    let json_path = local_dir.join("plugin.json");
    if let Ok(data) = std::fs::read_to_string(&json_path) {
        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&data) {
            let mut ch = get_channel_session()?;
            let _ = execute_ssh_command(
                &mut ch,
                &format!("chmod +x {}/{}", remote_dir, manifest.entry),
            );
        }
    }

    Ok(())
}

/// 서버에서 플러그인 삭제
pub fn uninstall_plugin(plugin_name: &str) -> Result<()> {
    let mut sftp = get_sftp_session()?;
    let remote_dir = format!("{}/{}", resolve_plugin_dir()?, plugin_name);
    rmrf_file(&mut sftp, Path::new(&remote_dir))?;

    // cron도 함께 제거
    let _ = unregister_cron(plugin_name);
    Ok(())
}

// ============================================================
// Enable / Disable
// ============================================================

/// 플러그인 활성화 (.disabled 마커 제거)
pub fn enable_plugin(plugin_name: &str) -> Result<()> {
    let sftp = get_sftp_session()?;
    let marker = format!("{}/{}/.disabled", resolve_plugin_dir()?, plugin_name);
    // 파일이 있으면 삭제, 없으면 무시
    let _ = sftp.unlink(Path::new(&marker));
    Ok(())
}

/// 플러그인 비활성화 (.disabled 마커 생성)
pub fn disable_plugin(plugin_name: &str) -> Result<()> {
    let sftp = get_sftp_session()?;
    let marker = format!("{}/{}/.disabled", resolve_plugin_dir()?, plugin_name);
    let mut file = sftp.create(Path::new(&marker))?;
    file.write_all(b"")?;

    // cron도 함께 해제
    let _ = unregister_cron(plugin_name);
    Ok(())
}

// ============================================================
// 실행
// ============================================================

/// Manual 플러그인 실행
pub fn execute_plugin(plugin_name: &str, input_json: &str) -> Result<PluginResult> {
    let hugo_config = get_hugo_config()?;

    let mut input: serde_json::Value = serde_json::from_str(input_json)
        .unwrap_or(serde_json::json!({}));
    input["context"] = serde_json::json!({
        "base_path": hugo_config.base_path,
        "content_paths": hugo_config.content_paths,
        "image_path": hugo_config.image_path,
        "hidden_path": hugo_config.hidden_path,
    });

    // manifest에서 entry 읽기
    let mut channel = get_channel_session()?;
    let entry_cmd = format!("cat {}/{}/plugin.json", PLUGIN_DIR, plugin_name);
    let manifest_str = execute_ssh_command(&mut channel, &entry_cmd)?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_str)
        .context("Failed to parse plugin.json")?;

    let escaped = serde_json::to_string(&input)?;
    let mut channel = get_channel_session()?;
    let cmd = format!(
        "printf '%s' '{}' | {}/{}/{}",
        escaped.replace('\'', "'\\''"),
        PLUGIN_DIR, plugin_name, manifest.entry
    );
    let output = execute_ssh_command(&mut channel, &cmd)?;

    let result: PluginResult = serde_json::from_str(&output)
        .context("Failed to parse plugin output")?;
    Ok(result)
}

/// Hook 이벤트에 등록된 **enabled** 플러그인만 priority 순으로 실행
pub fn run_hooks(event: HookEvent, data: serde_json::Value) -> Result<Vec<PluginResult>> {
    let server_plugins = discover_server_plugins().unwrap_or_default();
    let hugo_config = get_hugo_config()?;

    // 매칭되는 hook 수집: (priority, plugin_name, entry)
    let mut matched: Vec<(u32, String, String)> = Vec::new();
    for (plugin, enabled, _hash) in &server_plugins {
        if !enabled { continue; }

        for trigger in &plugin.triggers {
            if let Trigger::Hook { event: hook_event, priority } = trigger {
                if hook_event == &event {
                    matched.push((
                        priority.unwrap_or(50),
                        plugin.name.clone(),
                        plugin.entry.clone(),
                    ));
                }
            }
        }
    }

    // priority 오름차순, 동일 시 이름순
    matched.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut results = Vec::new();
    for (_, name, entry) in &matched {
        let input = serde_json::json!({
            "trigger": "hook",
            "event": format!("{:?}", event),
            "data": data,
            "context": {
                "base_path": hugo_config.base_path,
                "content_paths": hugo_config.content_paths,
                "image_path": hugo_config.image_path,
                "hidden_path": hugo_config.hidden_path,
            }
        });

        let mut channel = get_channel_session()?;
        let cmd = format!(
            "printf '%s' '{}' | {}/{}/{}",
            serde_json::to_string(&input)?.replace('\'', "'\\''"),
            PLUGIN_DIR, name, entry
        );

        match execute_ssh_command(&mut channel, &cmd) {
            Ok(output) => {
                if let Ok(result) = serde_json::from_str::<PluginResult>(&output) {
                    results.push(result);
                }
            }
            Err(e) => eprintln!("Hook plugin {} failed: {}", name, e),
        }
    }
    Ok(results)
}

// ============================================================
// Cron
// ============================================================

/// crontab 사용 가능 여부 확인
fn check_crontab_available() -> Result<()> {
    let mut channel = get_channel_session()?;
    let output = execute_ssh_command(&mut channel, "which crontab 2>/dev/null && echo OK")?;
    if !output.contains("OK") {
        bail!("crontab is not installed on the server");
    }
    Ok(())
}

pub fn register_cron(plugin_name: &str, schedule: &str, entry: &str, label: &str) -> Result<()> {
    check_crontab_available()?;
    let mut channel = get_channel_session()?;

    // entry 확장자에 따라 인터프리터 전체 경로 탐색
    let run_cmd = if entry.ends_with(".py") {
        let python_path = execute_ssh_command(
            &mut channel, "which python3 2>/dev/null || which python 2>/dev/null"
        )?.trim().to_string();
        if python_path.is_empty() {
            bail!("python3 not found on the server");
        }
        // get_channel_session은 매번 새 채널 필요
        channel = get_channel_session()?;
        format!("{} {}", python_path, entry)
    } else {
        format!("./{}", entry)
    };

    let marker = format!("inn-plugin:{}:{}", plugin_name, label);
    let job = format!(
        "{} cd {}/{} && {} # {}",
        schedule, PLUGIN_DIR, plugin_name, run_cmd, marker
    );
    let cmd = format!(
        "(crontab -l 2>/dev/null | grep -v '{marker}'; echo '{job}') | crontab -",
        marker = marker, job = job
    );
    execute_ssh_command(&mut channel, &cmd)?;
    Ok(())
}

/// 특정 라벨의 cron 제거 (개별 Off)
pub fn unregister_single_cron(plugin_name: &str, label: &str) -> Result<()> {
    check_crontab_available()?;
    let mut channel = get_channel_session()?;
    let marker = format!("inn-plugin:{}:{}", plugin_name, label);
    let cmd = format!(
        "crontab -l 2>/dev/null | grep -v '{}' | crontab -",
        marker
    );
    execute_ssh_command(&mut channel, &cmd)?;
    Ok(())
}

/// 등록된 cron 목록 조회 → "pluginName:label" 형태 반환
pub fn list_registered_crons() -> Result<Vec<String>> {
    let mut channel = get_channel_session()?;
    let output = execute_ssh_command(&mut channel, "crontab -l 2>/dev/null")?;
    let mut result = Vec::new();
    for line in output.lines() {
        // 마커 형식: # inn-plugin:{name}:{label}
        if let Some(pos) = line.find("inn-plugin:") {
            let marker = line[pos + "inn-plugin:".len()..].trim();
            if !marker.is_empty() {
                result.push(marker.to_string());
            }
        }
    }
    Ok(result)
}

/// 플러그인의 모든 cron 제거 (disable/uninstall 용)
pub fn unregister_cron(plugin_name: &str) -> Result<()> {
    let mut channel = get_channel_session()?;
    let marker = format!("inn-plugin:{}", plugin_name);
    let cmd = format!(
        "crontab -l 2>/dev/null | grep -v '{}' | crontab -",
        marker
    );
    execute_ssh_command(&mut channel, &cmd)?;
    Ok(())
}

// ============================================================
// Pull (서버 → 로컬)
// ============================================================

/// 서버에서 플러그인 전체를 로컬로 다운로드 (서버에서 tar → 단일 다운로드 → 로컬 해제)
pub fn pull_plugin(local_path: &str, plugin_name: &str) -> Result<()> {
    let sftp = get_sftp_session()?;
    let remote_base = resolve_plugin_dir()?;
    let remote_dir = format!("{}/{}", remote_base, plugin_name);
    let remote_tar = format!("{}/{}.tar.gz", remote_base, plugin_name);

    // 1. 서버에서 tar.gz 생성
    let mut channel = get_channel_session()?;
    execute_ssh_command(
        &mut channel,
        &format!("tar -czf '{}' -C '{}' .", remote_tar, remote_dir),
    )?;

    // 2. tar.gz 다운로드
    let mut remote_file = sftp.open(Path::new(&remote_tar))
        .context(format!("Failed to open remote tar: {}", remote_tar))?;
    let mut tar_data = Vec::new();
    remote_file.read_to_end(&mut tar_data)?;

    // 3. 서버 tar.gz 삭제
    let _ = sftp.unlink(Path::new(&remote_tar));

    // 4. 로컬 폴더 삭제 → tar.gz 해제
    let local_dir = Path::new(local_path).join(plugin_name);
    if local_dir.is_dir() {
        std::fs::remove_dir_all(&local_dir)?;
    }
    extract_tar_gz(&tar_data, &local_dir)?;

    Ok(())
}

// ============================================================
// SFTP 업로드/다운로드 헬퍼
// ============================================================

/// 로컬 디렉토리를 tar.gz 바이트로 압축
fn create_tar_gz(dir: &Path) -> Result<Vec<u8>> {
    let buf = Vec::new();
    let enc = flate2::write::GzEncoder::new(buf, flate2::Compression::fast());
    let mut tar = tar::Builder::new(enc);

    // 디렉토리 내부 파일만 추가 (.으로 시작하는 것 제외)
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_entry(|e| {
        !e.file_name().to_string_lossy().starts_with('.')
    }) {
        let entry = entry?;
        let rel_path = entry.path().strip_prefix(dir)?;
        if rel_path.as_os_str().is_empty() { continue; }
        if entry.file_type().is_file() {
            tar.append_path_with_name(entry.path(), rel_path)?;
        } else if entry.file_type().is_dir() {
            tar.append_dir(rel_path, entry.path())?;
        }
    }

    let enc = tar.into_inner()?;
    Ok(enc.finish()?)
}

/// tar.gz 바이트를 로컬 디렉토리에 해제
fn extract_tar_gz(data: &[u8], dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    let dec = flate2::read::GzDecoder::new(data);
    let mut archive = tar::Archive::new(dec);
    archive.unpack(dst)?;
    Ok(())
}

