use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;
use tauri::api::path::home_dir;
use tauri::InvokeError;

use crate::utils;

pub fn get_config_file_path() -> PathBuf {
    let home_dir = home_dir().unwrap();
    home_dir.join("cms_config.json")
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SshConfig {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub key_path: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct HugoConfig {
    #[serde(default)]
    pub hugo_cmd_path: String,
    #[serde(default)]
    pub base_path: String,
    #[serde(default)]
    pub content_path: String,
    #[serde(default)]
    pub image_path: String,
    #[serde(default)]
    pub config_path: String,
    #[serde(default)]
    pub layout_path: String,
    #[serde(default)]
    pub trashcan_path: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct AppConfig {
    #[serde(default)]
    pub ssh_config: SshConfig,
    #[serde(default)]
    pub hugo_config: HugoConfig,
}

#[tauri::command]
pub fn save_config(mut config: AppConfig) -> Result<(), InvokeError> {
    // 비밀번호 암호화
    if !config.ssh_config.password.is_empty() {
        config.ssh_config.password = utils::crypto::encrypt_string(&config.ssh_config.password)
            .map_err(|e| InvokeError::from(e.to_string()))?;
    }

    let config_file_path = get_config_file_path();
    let file = File::create(config_file_path).map_err(|e| InvokeError::from(e.to_string()))?;
    serde_json::to_writer_pretty(file, &config).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn load_config() -> Result<AppConfig, InvokeError> {
    let config_file_path = get_config_file_path();
    let file = File::open(config_file_path).map_err(|e| InvokeError::from(e.to_string()))?;
    let mut config: AppConfig =
        serde_json::from_reader(file).map_err(|e| InvokeError::from(e.to_string()))?;

    // 비밀번호 복호화
    if !config.ssh_config.password.is_empty() {
        config.ssh_config.password =
            utils::crypto::decrypt_string(&config.ssh_config.password).unwrap_or_default();
    }

    Ok(config)
}