use std::path::Path;
use std::io::prelude::*;
use anyhow::{Result, Context};
use crate::services::ssh_service::{get_channel_session, get_sftp_session, execute_ssh_command, get_server_home_path};
use crate::services::config_service::get_hugo_config;
use crate::services::file_service::mkdir_recursive;
use crate::types::plugin::*;

const PLUGIN_DIR: &str = "~/.inn_plugins";

/// ~ 를 실제 홈 경로로 치환
fn resolve_plugin_dir() -> Result<String> {
    let home = get_server_home_path()?;
    Ok(format!("{}/.inn_plugins", home))
}

/// 서버에서 플러그인 목록 조회
pub fn discover_plugins() -> Result<Vec<PluginManifest>> {
    let mut channel = get_channel_session()?;
    let list_cmd = format!(
        "for d in {}/*/plugin.json; do [ -f \"$d\" ] && cat \"$d\" && echo '---SEP---'; done",
        PLUGIN_DIR
    );
    let output = execute_ssh_command(&mut channel, &list_cmd)?;

    let mut plugins = Vec::new();
    for chunk in output.split("---SEP---") {
        let trimmed = chunk.trim();
        if trimmed.is_empty() { continue; }
        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(trimmed) {
            plugins.push(manifest);
        }
    }
    Ok(plugins)
}

/// Manual 플러그인 실행
pub fn execute_plugin(plugin_name: &str, input_json: &str) -> Result<PluginResult> {
    let hugo_config = get_hugo_config()?;

    // context 정보를 input에 병합
    let mut input: serde_json::Value = serde_json::from_str(input_json)
        .unwrap_or(serde_json::json!({}));
    input["context"] = serde_json::json!({
        "base_path": hugo_config.base_path,
        "content_path": hugo_config.content_path,
    });

    // 플러그인 manifest에서 entry 읽기
    let mut channel = get_channel_session()?;
    let entry_cmd = format!(
        "cat {}/{}/plugin.json",
        PLUGIN_DIR, plugin_name
    );
    let manifest_str = execute_ssh_command(&mut channel, &entry_cmd)?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_str)
        .context("Failed to parse plugin.json")?;

    // 스크립트 실행
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

/// Hook 이벤트에 등록된 플러그인 실행
pub fn run_hooks(event: HookEvent, data: serde_json::Value) -> Result<Vec<PluginResult>> {
    let plugins = discover_plugins()?;
    let hugo_config = get_hugo_config()?;
    let mut results = Vec::new();

    for plugin in &plugins {
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

/// 로컬 플러그인 디렉토리를 서버에 배포
pub fn deploy_plugins(local_path: &str) -> Result<Vec<String>> {
    let sftp = get_sftp_session()?;
    let remote_base = resolve_plugin_dir()?;

    // 서버에 ~/.inn_plugins 디렉토리 생성
    mkdir_recursive(&sftp, Path::new(&remote_base))?;

    let local = Path::new(local_path);
    if !local.is_dir() {
        anyhow::bail!("Local path is not a directory: {}", local_path);
    }

    let mut deployed = Vec::new();

    for entry in std::fs::read_dir(local)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() { continue; }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        // .git 등 숨김 디렉토리 스킵
        if dir_name.starts_with('.') { continue; }

        // plugin.json 존재 확인
        let plugin_json = entry.path().join("plugin.json");
        if !plugin_json.exists() { continue; }

        let remote_dir = format!("{}/{}", remote_base, dir_name);
        upload_dir_recursive(&sftp, &entry.path(), &remote_dir)?;

        // 실행 권한 부여 (entry 파일)
        if let Ok(data) = std::fs::read_to_string(&plugin_json) {
            if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&data) {
                let mut channel = get_channel_session()?;
                let _ = execute_ssh_command(
                    &mut channel,
                    &format!("chmod +x {}/{}", remote_dir, manifest.entry),
                );
            }
        }

        deployed.push(dir_name);
    }

    Ok(deployed)
}

/// 로컬 디렉토리를 서버에 재귀적으로 업로드
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

/// Cron 등록
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

/// Cron 해제
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
