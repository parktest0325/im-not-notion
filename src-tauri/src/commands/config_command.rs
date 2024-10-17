use tauri::command;
use tauri::InvokeError;
use crate::services::config_service::{load_app_config, save_app_config, get_app_config, set_app_config};
use crate::types::config::AppConfig;

#[command]
pub fn load_config() -> Result<AppConfig, InvokeError> {
    load_app_config().map_err(|e| InvokeError::from(e.to_string()))?;
    get_app_config().map_err(|e| InvokeError::from(e.to_string()))
}

#[command]
pub fn save_config(config: AppConfig) -> Result<(), InvokeError> {
    set_app_config(config).map_err(|e| InvokeError::from(e.to_string()))?;
    save_app_config().map_err(|e| InvokeError::from(e.to_string()))
}

#[command]
pub fn get_config() -> Result<AppConfig, InvokeError> {
    get_app_config().map_err(|e| InvokeError::from(e.to_string()))
}