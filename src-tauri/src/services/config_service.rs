use std::{path::Path, sync::Mutex};
use anyhow::{Result, Context};
use once_cell::sync::Lazy;
use crate::services::ssh_service::{connect_ssh_with_config, reconnect_ssh_with_config, get_sftp_session, get_server_home_path};
use crate::services::file_service::move_file;
use crate::types::config::{cms_config::HugoConfig, AppConfig, ClientConfig, CmsConfig};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::collections::HashMap;

static APP_CONFIG: Lazy<Mutex<Option<AppConfig>>> = Lazy::new(|| Mutex::new(None));

/// 설정 로드: 로컬 파일 → SSH 연결 시도 → 서버 설정 병합
pub fn load_app_config() -> Result<AppConfig> {
    // 1. 로컬 파일에서 읽기 (없으면 기본값)
    let client = ClientConfig::load_from_file().unwrap_or_default();

    let mut config = AppConfig {
        active_server: client.active_server.clone(),
        servers: client.servers.clone(),
        cms_config: CmsConfig::default(),
        shortcuts: HashMap::new(),
        plugin_local_path: client.plugin_local_path.clone(),
    };

    // 2. active server의 SSH 설정으로 연결 시도
    if let Some(ssh_config) = client.get_active_ssh_config() {
        if !ssh_config.host.is_empty() {
            if let Ok(()) = connect_ssh_with_config(&ssh_config) {
                // 3. 연결 성공하면 서버 설정 로드
                let sftp = get_sftp_session()?;
                let home_path = get_server_home_path()?;
                config.load_server_config(&sftp, &home_path)?;
            }
        }
    }

    // 4. 메모리에 저장
    *APP_CONFIG.lock().unwrap() = Some(config.clone());

    Ok(config)
}

/// 설정 저장: 로컬 저장 → SSH 연결 → 서버 저장 (비어있지 않으면)
pub fn save_app_config(mut new_config: AppConfig) -> Result<()> {
    // 1. 로컬에 먼저 저장
    new_config.save_client_config()?;

    // 2. active server로 SSH 연결 (설정 변경됐을 수 있으므로 강제 재연결)
    if let Some(ssh_config) = new_config.get_active_ssh_config() {
        reconnect_ssh_with_config(&ssh_config)?;

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
    }

    // 4. 메모리에 저장
    *APP_CONFIG.lock().unwrap() = Some(new_config);

    Ok(())
}

/// 서버 전환: servers 목록 업데이트 → active_server 변경 → 재연결 → 서버 설정 로드
/// servers를 함께 받아서 UI에서 새로 추가/수정한 서버 목록을 반영
pub fn switch_server(servers: Vec<crate::types::config::ServerEntry>, server_id: String) -> Result<AppConfig> {
    let mut config = get_app_config()?;
    config.servers = servers;
    config.active_server = server_id;

    // cms_config 초기화 (새 서버의 설정을 로드할 것이므로)
    config.cms_config = CmsConfig::default();
    config.shortcuts = HashMap::new();

    // 새 서버로 SSH 연결
    if let Some(ssh_config) = config.get_active_ssh_config() {
        reconnect_ssh_with_config(&ssh_config)?;
        let sftp = get_sftp_session()?;
        let home_path = get_server_home_path()?;
        config.load_server_config(&sftp, &home_path)?;
    }

    // 로컬에 저장 (서버 목록 + active_server)
    config.save_client_config()?;
    *APP_CONFIG.lock().unwrap() = Some(config.clone());
    Ok(config)
}

/// 플러그인 로컬 경로만 저장 (ClientConfig만 업데이트, SSH 재연결 없음)
pub fn save_plugin_local_path(path: String) -> Result<()> {
    let mut guard = APP_CONFIG.lock().unwrap();
    if let Some(ref mut config) = *guard {
        config.plugin_local_path = path;
        config.save_client_config()?;
    }
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
            .to_lowercase()
    } else {
        new_hidden_path.to_lowercase()
    };

    // 1) 현재 Hugo 설정을 가져온다. 없으면 빈 값으로 처리
    let (old_hidden_path, base_path) = match get_hugo_config() {
        Ok(cfg) => (cfg.hidden_path, cfg.base_path),
        Err(_) => (String::new(), String::new()),
    };

    if old_hidden_path.is_empty() || old_hidden_path == final_hidden {
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
