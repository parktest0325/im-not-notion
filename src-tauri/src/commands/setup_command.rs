use tauri::ipc::InvokeError;
use crate::services::setup_service::{
    self, PrerequisiteResult,
};
use crate::utils::IntoInvokeError;

#[tauri::command]
pub fn check_prerequisites_cmd() -> Result<PrerequisiteResult, InvokeError> {
    setup_service::check_prerequisites().into_invoke_err()
}

#[tauri::command]
pub fn check_hugo_installed_cmd() -> Result<Option<String>, InvokeError> {
    setup_service::check_hugo_installed().into_invoke_err()
}

#[tauri::command]
pub fn detect_server_platform_cmd() -> Result<(String, String), InvokeError> {
    setup_service::detect_server_platform().into_invoke_err()
}

#[tauri::command]
pub fn get_latest_hugo_version_cmd() -> Result<String, InvokeError> {
    setup_service::get_latest_hugo_version().into_invoke_err()
}

#[tauri::command]
pub async fn install_hugo_cmd(os: String, arch: String, version: String) -> Result<String, InvokeError> {
    // 다운로드/설치 동안 UI가 멈추지 않도록 blocking pool에서 실행
    tauri::async_runtime::spawn_blocking(move || {
        setup_service::install_hugo(&os, &arch, &version)
    })
    .await
    .map_err(|e| InvokeError::from(format!("Install task panicked: {}", e)))?
    .into_invoke_err()
}

#[tauri::command]
pub fn generate_site_name_cmd() -> Result<(String, String), InvokeError> {
    setup_service::generate_site_name().into_invoke_err()
}

#[tauri::command]
pub fn create_hugo_site_cmd(hugo_cmd_path: &str, site_path: &str) -> Result<(), InvokeError> {
    setup_service::create_hugo_site(hugo_cmd_path, site_path).into_invoke_err()
}

#[tauri::command]
pub fn validate_hugo_project_cmd(path: &str) -> Result<bool, InvokeError> {
    setup_service::validate_hugo_project(path).into_invoke_err()
}

#[tauri::command]
pub fn git_init_site_cmd(site_path: &str) -> Result<(), InvokeError> {
    setup_service::git_init_site(site_path).into_invoke_err()
}

#[tauri::command]
pub fn install_theme_cmd(theme_url: &str, site_path: &str) -> Result<String, InvokeError> {
    setup_service::install_theme(theme_url, site_path).into_invoke_err()
}
