use serde::{Deserialize, Serialize};
use std::fs::File;
use tauri::InvokeError;

pub const SETTING_FILE_PATH: &str = "./cms_config.json";

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SshConfig {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub key_path: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct HugoConfig {
    pub content_path: String,
    pub image_path: String,
    pub config_path: String,
    pub layout_path: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct AppConfig {
    #[serde(default)]
    pub ssh_config: SshConfig,
    #[serde(default)]
    pub hugo_config: HugoConfig,
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), InvokeError> {
    let file = File::create(SETTING_FILE_PATH).map_err(|e| InvokeError::from(e.to_string()))?; // std::io::Error를 InvokeError로 변환
    serde_json::to_writer_pretty(file, &config).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn load_config() -> Result<AppConfig, InvokeError> {
    let file = File::open(SETTING_FILE_PATH).map_err(|e| InvokeError::from(e.to_string()))?;
    let config: AppConfig =
        serde_json::from_reader(file).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(config)
}
