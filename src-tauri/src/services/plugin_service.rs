use anyhow::{Result, Context};
use crate::services::ssh_service::{get_channel_session, execute_ssh_command};
use crate::services::config_service::get_hugo_config;
use crate::types::plugin::*;

const PLUGIN_DIR: &str = "~/.inn_plugins";

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
