use tauri::InvokeError;
use crate::services::ssh_service::connect_ssh;
use crate::types::config::AppConfig;

#[tauri::command]
pub fn update_and_connect(config: AppConfig) -> Result<(), InvokeError> {
    connect_ssh(&config).map_err(|e| InvokeError::from(e.to_string()))
}