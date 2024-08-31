use std::sync::Mutex;
use anyhow::{Result, Context};
use once_cell::sync::Lazy;
use crate::types::config::{cms_config::HugoConfig, AppConfig};

static APP_CONFIG: Lazy<Mutex<Option<AppConfig>>> = Lazy::new(|| Mutex::new(None));

pub fn load_app_config() -> Result<()> {
    let mut config = AppConfig::default();
    config.load_config_from_file()?;
    *APP_CONFIG.lock().unwrap() = Some(config);
    Ok(())
}

pub fn save_app_config() -> Result<()> {
    APP_CONFIG.lock().unwrap()
        .as_ref()
        .context("APP_CONFIG not initialized")?
        .save_config_to_file()
}

pub fn get_app_config() -> Result<AppConfig> {
    APP_CONFIG.lock().unwrap()
        .clone()
        .context("APP_CONFIG not initialized")
}

pub fn set_app_config(new_config: AppConfig) -> Result<()> {
    *APP_CONFIG.lock().unwrap() = Some(new_config);
    save_app_config()
}

// 임시로 hugo 만 사용하는 현재 상황에 맞춰서 구현
pub fn get_hugo_config() -> Result<HugoConfig> {
    Ok(get_app_config()?.cms_config.hugo_config)
}