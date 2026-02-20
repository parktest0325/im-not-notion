use crate::services::file_service::{self, FileSystemNode};
use crate::utils::IntoInvokeError;
use tauri::ipc::InvokeError;

#[tauri::command]
pub fn get_file_tree() -> Result<FileSystemNode, InvokeError> {
    file_service::build_file_tree().into_invoke_err()
}

#[tauri::command]
pub fn get_file_content(file_path: &str) -> Result<String, InvokeError> {
    file_service::read_content(file_path).into_invoke_err()
}

#[tauri::command]
pub fn save_file_content(file_path: &str, file_data: &str, manual: bool) -> Result<(), InvokeError> {
    file_service::write_content(file_path, file_data, manual).into_invoke_err()
}

#[tauri::command]
pub fn save_file_image(
    file_path: &str,
    file_name: &str,
    file_data: Vec<u8>,
) -> Result<String, InvokeError> {
    file_service::write_image(file_path, file_name, file_data).into_invoke_err()
}

#[tauri::command]
pub fn new_content_for_hugo(file_path: &str) -> Result<String, InvokeError> {
    file_service::create_content(file_path).into_invoke_err()
}

#[tauri::command]
pub fn remove_file(path: &str) -> Result<(), InvokeError> {
    file_service::remove_content(path).into_invoke_err()
}

#[tauri::command]
pub fn move_file_or_folder(src: &str, dst: &str) -> Result<(), InvokeError> {
    file_service::move_content(src, dst).into_invoke_err()
}

#[tauri::command]
pub fn toggle_hidden_file(path: &str, state: bool) -> Result<(), InvokeError> {
    file_service::toggle_hidden(path, state).into_invoke_err()
}

#[tauri::command]
pub fn check_file_hidden(path: &str) -> Result<bool, InvokeError> {
    file_service::check_hidden(path).into_invoke_err()
}
