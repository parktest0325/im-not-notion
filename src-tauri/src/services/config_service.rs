use std::{path::Path, sync::Mutex};
use anyhow::{Result, Context};
use once_cell::sync::Lazy;
use crate::{services::{get_sftp_session, move_file}, types::config::{cms_config::HugoConfig, AppConfig}};

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
    set_hidden_path(&new_config.cms_config.hugo_config.hidden_path);
    *APP_CONFIG.lock().unwrap() = Some(new_config);
    save_app_config()
}

// 임시로 hugo 만 사용하는 현재 상황에 맞춰서 구현
pub fn get_hugo_config() -> Result<HugoConfig> {
    Ok(get_app_config()?.cms_config.hugo_config)
}

pub fn set_hidden_path(new_hidden_path: &str) -> Result<()> {
    // 1) 현재 Hugo 설정을 가져온다.
    let hugo_config = get_hugo_config()?;
    if hugo_config.hidden_path == new_hidden_path {
        // 이미 같은 경로라면 아무 것도 안 함
        return Ok(());
    }

    // 2) 절대 경로 계산
    let base = &hugo_config.base_path;
    let old_hidden_abs = format!("{}/content/{}", base, hugo_config.hidden_path);
    let new_hidden_abs = format!("{}/content/{}", base, new_hidden_path);

    // 3) 디렉터리 이동 (SFTP)
    let sftp = get_sftp_session()?;
    move_file(&sftp, Path::new(&old_hidden_abs), Path::new(&new_hidden_abs))?;

    // 4) AppConfig 사본을 만들어 hidden_path 만 바꾼다.
    let mut new_cfg = get_app_config()?;
    new_cfg.cms_config.hugo_config.hidden_path = new_hidden_path.to_string();


    Ok(())
}