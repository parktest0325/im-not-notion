use crate::{
    setting::AppConfig,
    ssh::sftp::{list_directory, FileSystemNode},
};
use ssh2::{Session, Sftp};
use std::{net::TcpStream, path::Path, sync::Mutex};
use tauri::InvokeError;

static APP_CONFIG: Mutex<Option<AppConfig>> = Mutex::new(None);
static SSH_CLIENT: Mutex<Option<Session>> = Mutex::new(None);

#[tauri::command]
pub fn update_and_connect(config: AppConfig) -> Result<(), InvokeError> {
    // 전역변수 업데이트 이전에 연결등에서 에러가 발생하면 업데이트하지 않음
    let mut session = Session::new().map_err(|e| InvokeError::from(e.to_string()))?;
    let tcp = TcpStream::connect(format!(
        "{}:{}",
        config.ssh_config.host, config.ssh_config.port
    ))
    .map_err(|e| InvokeError::from(e.to_string()))?;
    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| InvokeError::from(e.to_string()))?;

    if !config.ssh_config.password.is_empty() {
        session
            .userauth_password(&config.ssh_config.username, &config.ssh_config.password)
            .map_err(|e| InvokeError::from(e.to_string()))?;
    } else {
        session
            .userauth_pubkey_file(
                &config.ssh_config.username,
                None,
                Path::new(&config.ssh_config.key_path),
                None,
            )
            .map_err(|e| InvokeError::from(e.to_string()))?;
    }

    // 전역변수 업데이트
    let mut app_config = APP_CONFIG.lock().unwrap();
    *app_config = Some(config);

    let mut ssh_client = SSH_CLIENT.lock().unwrap();
    *ssh_client = Some(session);

    Ok(())
}

#[tauri::command]
pub fn get_file_list() -> Result<FileSystemNode, InvokeError> {
    let ssh_client_lock = SSH_CLIENT
        .lock()
        .map_err(|e| InvokeError::from(e.to_string()))?;
    let session = ssh_client_lock
        .as_ref()
        .ok_or_else(|| InvokeError::from("SSH session not initialized"))?;

    // SFTP 세션을 시작합니다.
    let sftp: Sftp = session
        .sftp()
        .map_err(|e| InvokeError::from(e.to_string()))?;

    // AppConfig에서 content_path 경로를 가져옵니다.
    let app_config_lock = APP_CONFIG
        .lock()
        .map_err(|e| InvokeError::from(e.to_string()))?;
    let app_config = app_config_lock
        .as_ref()
        .ok_or_else(|| InvokeError::from("App config not initialized"))?;
    let content_path = &app_config.hugo_config.content_path;

    // 지정된 경로의 파일 리스트를 조회합니다.
    let file_list = list_directory(&sftp, Path::new(content_path), 5)
        .map_err(|e| InvokeError::from(e.to_string()))?;

    Ok(file_list)
}

// get content
