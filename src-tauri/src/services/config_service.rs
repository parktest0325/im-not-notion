use std::{path::Path, sync::Mutex};
use anyhow::{Result, Context};
use once_cell::sync::Lazy;
use crate::{services::{execute_ssh_command, get_channel_session, get_sftp_session, move_file}, types::config::{cms_config::HugoConfig, AppConfig}};
use crate::services::ssh_service::{connect_ssh, reconnect_ssh};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

static APP_CONFIG: Lazy<Mutex<Option<AppConfig>>> = Lazy::new(|| Mutex::new(None));

/// SSH 서버의 홈 디렉토리 경로를 가져옴
fn get_server_home_path() -> Result<String> {
    let mut channel = get_channel_session()?;
    let output = execute_ssh_command(&mut channel, "echo $HOME")?;
    Ok(output.trim().to_string())
}

/// 설정 로드: 로컬 파일 → SSH 연결 시도 → 서버 설정 병합
pub fn load_app_config() -> Result<AppConfig> {
    // 1. 로컬 파일에서 읽기 (없으면 기본값)
    let mut config = AppConfig::load_client_only().unwrap_or_default();

    // 2. SSH 설정이 있으면 연결 시도
    if config.has_ssh_config() {
        if let Ok(()) = connect_ssh(&config) {
            // 3. 연결 성공하면 서버 설정 로드
            let sftp = get_sftp_session()?;
            let home_path = get_server_home_path()?;
            config.load_server_config(&sftp, &home_path)?;
        }
    }

    // 4. 메모리에 저장
    *APP_CONFIG.lock().unwrap() = Some(config.clone());

    Ok(config)
}

/// 설정 저장: 로컬 저장 → SSH 연결 → 서버 저장 (비어있지 않으면)
pub fn save_app_config(mut new_config: AppConfig) -> Result<()> {
    // 1. 로컬에 먼저 저장 (SSH 설정)
    new_config.save_client_config()?;

    // 2. SSH 연결 (설정 변경됐을 수 있으므로 강제 재연결)
    reconnect_ssh(&new_config)?;

    // 3. 서버 설정이 비어있지 않으면 저장
    if !new_config.cms_config.hugo_config.is_empty() {
        // hidden_path 처리
        new_config.cms_config.hugo_config.hidden_path
            = set_hidden_path(new_config.cms_config.hugo_config.hidden_path.trim())?;

        // 서버에 저장
        let sftp = get_sftp_session()?;
        let home_path = get_server_home_path()?;
        new_config.save_server_config(&sftp, &home_path)?;
    }

    // 4. 메모리에 저장
    *APP_CONFIG.lock().unwrap() = Some(new_config);

    Ok(())
}

pub fn get_app_config() -> Result<AppConfig> {
    APP_CONFIG.lock().unwrap()
        .clone()
        .context("APP_CONFIG not initialized")
}

pub fn get_hugo_config() -> Result<HugoConfig> {
    APP_CONFIG.lock().unwrap()
        .clone()
        .context("APP_CONFIG not initialized")
        .map(|c| c.cms_config.hugo_config)
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

    // 1) 현재 Hugo 설정을 가져온다. 없으면 빈 값으로 처리
    let (old_hidden_path, base_path) = match get_hugo_config() {
        Ok(cfg) => (cfg.hidden_path, cfg.base_path),
        Err(_) => (String::new(), String::new()),
    };

    if old_hidden_path.is_empty() || old_hidden_path == final_hidden {
        // 이미 같은 경로 or
        // 이전 설정이 없는경우에도 move를 안해도됨. hidden 파일이 없다는 뜻이니까
        return Ok(final_hidden);
    }

    // 2) 절대 경로 계산
    let old_hidden_abs = format!("{}/content/{}", base_path, old_hidden_path);
    let new_hidden_abs = format!("{}/content/{}", base_path, final_hidden);

    // 3) 디렉터리 이동 (SFTP) - 대상이 이미 존재하면 건너뜀
    let sftp = get_sftp_session()?;
    let new_path = Path::new(&new_hidden_abs);

    if sftp.stat(new_path).is_ok() {
        return Ok(final_hidden);
    }

    // 소스가 존재할 때만 이동
    let old_path = Path::new(&old_hidden_abs);
    if sftp.stat(old_path).is_ok() {
        move_file(&sftp, old_path, new_path)?;
    }

    Ok(final_hidden)
}