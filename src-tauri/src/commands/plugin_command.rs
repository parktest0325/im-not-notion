use crate::services::plugin_service;
use crate::types::plugin::{PluginInfo, PluginResult};
use crate::utils::IntoInvokeError;
use tauri::ipc::InvokeError;

#[tauri::command]
pub fn list_plugins(local_path: &str) -> Result<Vec<PluginInfo>, InvokeError> {
    plugin_service::list_all_plugins(local_path).into_invoke_err()
}

#[tauri::command]
pub fn install_plugin(local_path: &str, name: &str) -> Result<(), InvokeError> {
    plugin_service::install_plugin(local_path, name).into_invoke_err()
}

#[tauri::command]
pub fn uninstall_plugin(name: &str) -> Result<(), InvokeError> {
    plugin_service::uninstall_plugin(name).into_invoke_err()
}

#[tauri::command]
pub fn enable_plugin(name: &str) -> Result<(), InvokeError> {
    plugin_service::enable_plugin(name).into_invoke_err()
}

#[tauri::command]
pub fn disable_plugin(name: &str) -> Result<(), InvokeError> {
    plugin_service::disable_plugin(name).into_invoke_err()
}

#[tauri::command]
pub fn run_plugin(name: &str, input: &str) -> Result<PluginResult, InvokeError> {
    plugin_service::execute_plugin(name, input).into_invoke_err()
}

#[tauri::command]
pub fn register_plugin_cron(name: &str, schedule: &str, entry: &str) -> Result<(), InvokeError> {
    plugin_service::register_cron(name, schedule, entry).into_invoke_err()
}

#[tauri::command]
pub fn unregister_plugin_cron(name: &str) -> Result<(), InvokeError> {
    plugin_service::unregister_cron(name).into_invoke_err()
}

#[tauri::command]
pub fn pull_plugin(local_path: &str, name: &str) -> Result<(), InvokeError> {
    plugin_service::pull_plugin(local_path, name).into_invoke_err()
}

#[tauri::command]
pub fn open_plugin_in_editor(local_path: &str, name: &str) -> Result<(), InvokeError> {
    let dir = std::path::Path::new(local_path).join(name);
    let dir_str = dir.to_string_lossy().to_string();
    // Try VS Code first, fallback to system open
    if std::process::Command::new("code").arg(&dir_str).spawn().is_ok() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&dir_str).spawn()
        .map_err(|e| InvokeError::from(e.to_string()))?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer").arg(&dir_str).spawn()
        .map_err(|e| InvokeError::from(e.to_string()))?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(&dir_str).spawn()
        .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}
