use crate::{
    setting::{AppConfig, HugoConfig},
    ssh::sftp::{
        get_file, list_directory, mkdir_recursive, move_file, new_hugo_content, save_file,
        save_image, FileSystemNode,
    },
};

use ssh2::{Channel, Session, Sftp};
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
    let sftp: Sftp = get_global_sftp_session()?;

    let hugo_config = get_global_hugo_config()?;
    let content_path = &hugo_config.content_path;

    // 지정된 경로의 파일 리스트를 조회합니다.
    let file_list = list_directory(&sftp, Path::new(content_path), 5)
        .map_err(|e| InvokeError::from(e.to_string()))?;

    Ok(file_list)
}

#[tauri::command]
pub fn get_file_content(file_path: &str) -> Result<String, InvokeError> {
    let sftp: Sftp = get_global_sftp_session()?;

    let hugo_config = get_global_hugo_config()?;
    let content_path = &hugo_config.content_path;

    let file_data = get_file(&sftp, Path::new(&format!("{}{}", content_path, file_path)))
        .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(file_data)
}

#[tauri::command]
pub fn save_file_content(file_path: &str, file_data: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_global_sftp_session()?;

    let hugo_config = get_global_hugo_config()?;
    let content_path = &hugo_config.content_path;

    save_file(
        &sftp,
        Path::new(&format!("{}{}", content_path, file_path)),
        file_data.to_string(),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn save_file_image(
    file_path: &str,
    file_name: &str,
    file_data: Vec<u8>,
) -> Result<String, InvokeError> {
    let sftp: Sftp = get_global_sftp_session()?;

    let hugo_config = get_global_hugo_config()?;
    let image_path = &hugo_config.image_path;

    let image_ext = "";
    let ret_path = format!("{}/{}{}", file_path, file_name, image_ext);

    save_image(
        &sftp,
        Path::new(&format!("{}{}", image_path, ret_path)),
        file_data,
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(ret_path)
}

#[tauri::command]
pub fn new_content_for_hugo(file_path: &str) -> Result<(), InvokeError> {
    let mut channel = get_global_channel_session()?;
    let hugo_config = get_global_hugo_config()?;

    new_hugo_content(
        &mut channel,
        &hugo_config.base_path,
        &hugo_config.hugo_cmd_path,
        &format!("{}{}", &hugo_config.content_path, file_path),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn move_file_and_folder(src: &str, dst: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_global_sftp_session()?;
    let hugo_config = get_global_hugo_config()?;

    move_file(
        &sftp,
        &Path::new(&format!("{}{}", &hugo_config.content_path, src)),
        &Path::new(&format!("{}{}", &hugo_config.content_path, dst)),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn move_to_trashcan(src: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_global_sftp_session()?;
    let hugo_config = get_global_hugo_config()?;

    move_file(
        &sftp,
        &Path::new(&format!("{}{}", &hugo_config.content_path, src)),
        &Path::new(&format!("{}{}", &hugo_config.trashcan_path, src)),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn make_directory(path: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_global_sftp_session()?;
    let hugo_config = get_global_hugo_config()?;
    mkdir_recursive(
        &sftp,
        &Path::new(&format!("{}{}", &hugo_config.content_path, path)),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

fn get_global_channel_session() -> Result<Channel, InvokeError> {
    let ssh_client_lock = SSH_CLIENT
        .lock()
        .map_err(|e| InvokeError::from(e.to_string()))?;
    let session = ssh_client_lock
        .as_ref()
        .ok_or_else(|| InvokeError::from("SSH session not initialized"))?;
    let channel = session
        .channel_session()
        .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(channel)
}

fn get_global_sftp_session() -> Result<Sftp, InvokeError> {
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
    Ok(sftp)
}

fn get_global_hugo_config() -> Result<HugoConfig, InvokeError> {
    let app_config_lock = APP_CONFIG
        .lock()
        .map_err(|e| InvokeError::from(e.to_string()))?;
    let app_config = app_config_lock
        .as_ref()
        .ok_or_else(|| InvokeError::from("App config not initialized"))?;
    Ok(app_config.hugo_config.clone())
}
