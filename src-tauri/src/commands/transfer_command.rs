use tauri::{AppHandle, ipc::InvokeError};

use crate::services::transfer_service::{
    check_download_conflicts as svc_check_download,
    check_upload_conflicts as svc_check_upload,
    download_to_local as svc_download,
    upload_to_remote as svc_upload,
    ConflictItem, ConflictPolicy,
};
use crate::services::fs_service::{
    delete_remote as svc_delete_remote, get_home_dir_remote,
    list_remote_dir as svc_list_remote, mkdir_remote as svc_mkdir_remote,
    move_remote as svc_move_remote, FsEntry,
};

fn to_invoke<E: std::fmt::Display>(e: E) -> InvokeError {
    InvokeError::from(e.to_string())
}

/* ===== Transfer ===== */

#[tauri::command]
pub fn check_upload_conflicts(
    local_paths: Vec<String>,
    remote_dir: String,
) -> Result<Vec<ConflictItem>, InvokeError> {
    svc_check_upload(&local_paths, &remote_dir).map_err(to_invoke)
}

#[tauri::command]
pub fn check_download_conflicts(
    remote_paths: Vec<String>,
    local_dir: String,
) -> Result<Vec<ConflictItem>, InvokeError> {
    svc_check_download(&remote_paths, &local_dir).map_err(to_invoke)
}

#[tauri::command]
pub fn upload_to_remote(
    local_paths: Vec<String>,
    remote_dir: String,
    policy: ConflictPolicy,
    app: AppHandle,
) -> Result<String, InvokeError> {
    svc_upload(local_paths, remote_dir, policy, app).map_err(to_invoke)
}

#[tauri::command]
pub fn download_to_local(
    remote_paths: Vec<String>,
    local_dir: String,
    policy: ConflictPolicy,
    app: AppHandle,
) -> Result<String, InvokeError> {
    svc_download(remote_paths, local_dir, policy, app).map_err(to_invoke)
}

/* ===== Remote file operations ===== */

#[tauri::command]
pub fn list_remote_dir(path: String) -> Result<Vec<FsEntry>, InvokeError> {
    svc_list_remote(&path).map_err(to_invoke)
}

#[tauri::command]
pub fn delete_remote_paths(paths: Vec<String>) -> Result<(), InvokeError> {
    svc_delete_remote(&paths).map_err(to_invoke)
}

#[tauri::command]
pub fn move_remote_path(src: String, dst: String) -> Result<(), InvokeError> {
    svc_move_remote(&src, &dst).map_err(to_invoke)
}

#[tauri::command]
pub fn mkdir_remote_path(path: String) -> Result<(), InvokeError> {
    svc_mkdir_remote(&path).map_err(to_invoke)
}

#[tauri::command]
pub fn remote_home_dir() -> Result<String, InvokeError> {
    get_home_dir_remote().map_err(to_invoke)
}
