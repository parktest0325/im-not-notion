use crate::services::plugin_service;
use crate::types::plugin::{PluginManifest, PluginResult};
use crate::utils::IntoInvokeError;
use tauri::ipc::InvokeError;

#[tauri::command]
pub fn list_plugins() -> Result<Vec<PluginManifest>, InvokeError> {
    plugin_service::discover_plugins().into_invoke_err()
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
pub fn deploy_plugins(local_path: &str) -> Result<Vec<String>, InvokeError> {
    plugin_service::deploy_plugins(local_path).into_invoke_err()
}
