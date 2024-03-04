use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use tauri::InvokeError;

pub const SETTING_FILE_PATH: &str = "./cms_config.json";

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SshClientConfig {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub key_path: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct AppConfig {
    pub ssh_client: SshClientConfig,
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), InvokeError> {
    let file = File::create(SETTING_FILE_PATH).map_err(|e| InvokeError::from(e.to_string()))?; // std::io::Error를 InvokeError로 변환
    serde_json::to_writer_pretty(file, &config).map_err(|e| InvokeError::from(e.to_string()))?; // serde_json::Error를 InvokeError로 변환
    Ok(())
}

#[tauri::command]
pub fn load_config() -> Result<AppConfig, InvokeError> {
    let file = File::open(SETTING_FILE_PATH).map_err(|e| InvokeError::from(e.to_string()))?; // std::io::Error를 InvokeError로 변환
    let config: AppConfig =
        serde_json::from_reader(file).map_err(|e| InvokeError::from(e.to_string()))?; // serde_json::Error를 InvokeError로 변환
    Ok(config)
}
