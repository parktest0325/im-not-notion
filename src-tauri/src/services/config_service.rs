use std::{path::Path, sync::Mutex};
use anyhow::{Result, Context};
use once_cell::sync::Lazy;
use crate::{services::{get_sftp_session, move_file}, types::config::{cms_config::HugoConfig, AppConfig}};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

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

pub fn set_app_config(mut new_config: AppConfig) -> Result<()> {
    new_config.cms_config.hugo_config.hidden_path
        = set_hidden_path(new_config.cms_config.hugo_config.hidden_path.trim())?;
    *APP_CONFIG.lock().unwrap() = Some(new_config);
    save_app_config()
}

// 임시로 hugo 만 사용하는 현재 상황에 맞춰서 구현
pub fn get_hugo_config() -> Result<HugoConfig> {
    Ok(get_app_config()?.cms_config.hugo_config)
}

// 이전 설정 없을때 -> 새로운 설정도 없을때  : 랜덤하게 hidden 생성. move_file 없음
//                -> 새로운 설정 있을때   : 설정한 값으로 생성. move_file 없음
// 이전 설정 있을때 -> 새로운 설정도 없을때  : 랜덤하게 hidden 생성. 생성한 폴더로 move_file
//                -> 새로운 설정 있을때   : 설정한 값으로 생성. 생성한 폴더로 move_file
// 값이 동일할때 : 아무작업 안함
pub fn set_hidden_path(new_hidden_path: &str) -> Result<String> {
    let final_hidden = if new_hidden_path.is_empty() {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect::<String>()
    } else {
        new_hidden_path.to_string()
    };
    
    // 1) 현재 Hugo 설정을 가져온다. (+ 잠금 즉시 해제)
    let (old_hidden_path, base_path) = {
        let cfg = get_hugo_config()?;
        (cfg.hidden_path, cfg.base_path)
    };
    if old_hidden_path.is_empty() || old_hidden_path == final_hidden {
        // 이미 같은 경로 or
        // 이전 설정이 없는경우에도 move를 안해도됨. hidden 파일이 없다는 뜻이니까
        println!("same path");
        return Ok(final_hidden);
    }

    // 2) 절대 경로 계산
    let old_hidden_abs = format!("{}/content/{}", base_path, old_hidden_path);
    let new_hidden_abs = format!("{}/content/{}", base_path, final_hidden);
    println!("{old_hidden_abs} -> {new_hidden_abs}");

    // 3) 디렉터리 이동 (SFTP)
    let sftp = get_sftp_session()?;
    move_file(&sftp, Path::new(&old_hidden_abs), Path::new(&new_hidden_abs))?;

    Ok(final_hidden)
}