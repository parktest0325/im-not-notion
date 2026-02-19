use std::path::Path;
use std::io::prelude::*;
use std::collections::HashMap;
use anyhow::{Result, Context};
use crate::services::ssh_service::{get_channel_session, get_sftp_session, execute_ssh_command, get_server_home_path};
use crate::services::config_service::get_hugo_config;
use crate::services::file_service::{mkdir_recursive, rmrf_file};
use crate::types::plugin::*;

const PLUGIN_DIR: &str = "~/.inn_plugins";

/// ~ 를 실제 홈 경로로 치환
fn resolve_plugin_dir() -> Result<String> {
    let home = get_server_home_path()?;
    Ok(format!("{}/.inn_plugins", home))
}

// ============================================================
// 조회
// ============================================================

/// 서버에 설치된 플러그인 목록 (enabled 상태 포함)
fn discover_server_plugins() -> Result<Vec<(PluginManifest, bool)>> {
    let mut channel = get_channel_session()?;
    // plugin.json 내용 + .disabled 존재 여부를 한 번에 조회
    let cmd = format!(
        "for d in {0}/*/plugin.json; do \
            [ -f \"$d\" ] || continue; \
            dir=$(dirname \"$d\"); \
            name=$(basename \"$dir\"); \
            disabled=\"false\"; \
            [ -f \"$dir/.disabled\" ] && disabled=\"true\"; \
            echo \"---ENTRY---\"; \
            echo \"$disabled\"; \
            cat \"$d\"; \
        done",
        PLUGIN_DIR
    );
    let output = execute_ssh_command(&mut channel, &cmd)?;

    let mut plugins = Vec::new();
    for chunk in output.split("---ENTRY---") {
        let trimmed = chunk.trim();
        if trimmed.is_empty() { continue; }

        // 첫 줄: disabled 여부, 나머지: plugin.json
        let mut lines = trimmed.splitn(2, '\n');
        let disabled_str = lines.next().unwrap_or("false").trim();
        let json_str = lines.next().unwrap_or("").trim();

        if json_str.is_empty() { continue; }
        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(json_str) {
            let enabled = disabled_str != "true";
            plugins.push((manifest, enabled));
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

/// 로컬 + 서버 플러그인 병합 리스트
pub fn list_all_plugins(local_path: &str) -> Result<Vec<PluginInfo>> {
    let server_plugins = discover_server_plugins().unwrap_or_default();
    let local_plugins = discover_local_plugins(local_path).unwrap_or_default();

    // 서버 플러그인을 name → (manifest, enabled) 맵으로
    let mut server_map: HashMap<String, (PluginManifest, bool)> = HashMap::new();
    for (manifest, enabled) in server_plugins {
        server_map.insert(manifest.name.clone(), (manifest, enabled));
    }

    let mut result: Vec<PluginInfo> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 로컬 플러그인 우선 순회
    for local_manifest in &local_plugins {
        seen.insert(local_manifest.name.clone());
        if let Some((_, enabled)) = server_map.get(&local_manifest.name) {
            // 로컬 + 서버 양쪽 존재 → installed, 서버 enabled 상태 사용
            result.push(PluginInfo {
                manifest: local_manifest.clone(),
                installed: true,
                enabled: *enabled,
            });
        } else {
            // 로컬에만 존재 → not installed
            result.push(PluginInfo {
                manifest: local_manifest.clone(),
                installed: false,
                enabled: false,
            });
        }
    }

    // 서버에만 있는 플러그인
    for (name, (manifest, enabled)) in &server_map {
        if !seen.contains(name) {
            result.push(PluginInfo {
                manifest: manifest.clone(),
                installed: true,
                enabled: *enabled,
            });
        }
    }

    Ok(result)
}

// ============================================================
// Install / Uninstall
// ============================================================

/// 로컬 플러그인 하나를 서버에 설치
pub fn install_plugin(local_path: &str, plugin_name: &str) -> Result<()> {
    let sftp = get_sftp_session()?;
    let remote_base = resolve_plugin_dir()?;
    mkdir_recursive(&sftp, Path::new(&remote_base))?;

    let local_dir = Path::new(local_path).join(plugin_name);
    if !local_dir.is_dir() {
        anyhow::bail!("Plugin not found locally: {}", plugin_name);
    }

    let remote_dir = format!("{}/{}", remote_base, plugin_name);
    upload_dir_recursive(&sftp, &local_dir, &remote_dir)?;

    // 실행 권한 부여
    let json_path = local_dir.join("plugin.json");
    if let Ok(data) = std::fs::read_to_string(&json_path) {
        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&data) {
            let mut channel = get_channel_session()?;
            let _ = execute_ssh_command(
                &mut channel,
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
        "content_path": hugo_config.content_path,
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

/// Hook 이벤트에 등록된 **enabled** 플러그인만 실행
pub fn run_hooks(event: HookEvent, data: serde_json::Value) -> Result<Vec<PluginResult>> {
    let server_plugins = discover_server_plugins().unwrap_or_default();
    let hugo_config = get_hugo_config()?;
    let mut results = Vec::new();

    for (plugin, enabled) in &server_plugins {
        if !enabled { continue; }

        for trigger in &plugin.triggers {
            if let Trigger::Hook { event: hook_event } = trigger {
                if *hook_event == event {
                    let input = serde_json::json!({
                        "trigger": "hook",
                        "event": format!("{:?}", event),
                        "data": data,
                        "context": {
                            "base_path": hugo_config.base_path,
                            "content_path": hugo_config.content_path,
                        }
                    });

                    let mut channel = get_channel_session()?;
                    let cmd = format!(
                        "printf '%s' '{}' | {}/{}/{}",
                        serde_json::to_string(&input)?.replace('\'', "'\\''"),
                        PLUGIN_DIR, plugin.name, plugin.entry
                    );

                    match execute_ssh_command(&mut channel, &cmd) {
                        Ok(output) => {
                            if let Ok(result) = serde_json::from_str::<PluginResult>(&output) {
                                results.push(result);
                            }
                        }
                        Err(e) => eprintln!("Hook plugin {} failed: {}", plugin.name, e),
                    }
                }
            }
        }
    }
    Ok(results)
}

// ============================================================
// Cron
// ============================================================

pub fn register_cron(plugin_name: &str, schedule: &str, entry: &str) -> Result<()> {
    let mut channel = get_channel_session()?;
    let marker = format!("inn-plugin:{}", plugin_name);
    let job = format!(
        "{} cd {}/{} && ./{} # {}",
        schedule, PLUGIN_DIR, plugin_name, entry, marker
    );
    let cmd = format!(
        "(crontab -l 2>/dev/null | grep -v '{}'; echo '{}') | crontab -",
        marker, job
    );
    execute_ssh_command(&mut channel, &cmd)?;
    Ok(())
}

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
// SFTP 업로드 헬퍼
// ============================================================

fn upload_dir_recursive(sftp: &ssh2::Sftp, local_dir: &Path, remote_dir: &str) -> Result<()> {
    mkdir_recursive(sftp, Path::new(remote_dir))?;

    for entry in std::fs::read_dir(local_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }

        let remote_path = format!("{}/{}", remote_dir, name);

        if entry.file_type()?.is_dir() {
            upload_dir_recursive(sftp, &entry.path(), &remote_path)?;
        } else {
            let data = std::fs::read(entry.path())?;
            let mut remote_file = sftp.create(Path::new(&remote_path))?;
            remote_file.write_all(&data)?;
        }
    }
    Ok(())
}
