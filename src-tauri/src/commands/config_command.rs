use tauri::command;
use tauri::ipc::InvokeError;
use crate::services::config_service::{load_app_config, save_app_config, get_app_config, set_app_config};
use crate::types::config::AppConfig;
use crate::utils::IntoInvokeError;

#[command]
pub fn load_config() -> Result<AppConfig, InvokeError> {
    load_app_config().into_invoke_err()?;
    get_app_config().into_invoke_err()
}

#[command]
pub fn save_config(config: AppConfig) -> Result<(), InvokeError> {
    set_app_config(config).into_invoke_err()?;
    Ok(())
    // save_app_config().into_invoke_err()
}

#[command]
pub fn get_config() -> Result<AppConfig, InvokeError> {
    get_app_config().into_invoke_err()
}