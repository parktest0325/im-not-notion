use tauri::command;
use tauri::ipc::InvokeError;
use crate::services::config_service::{load_app_config, save_app_config};
use crate::types::config::AppConfig;
use crate::utils::IntoInvokeError;

/// 설정 로드: 로컬 + SSH 연결되어 있으면 서버 설정도 병합
#[command]
pub fn load_config() -> Result<AppConfig, InvokeError> {
    load_app_config().into_invoke_err()
}

/// 설정 저장: 로컬 저장 → SSH 연결 → 서버 저장
#[command]
pub fn save_config(config: AppConfig) -> Result<(), InvokeError> {
    save_app_config(config).into_invoke_err()
}