use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use ssh2::Sftp;
use std::io::{Read, Write};
use std::path::Path;

use super::CmsConfig;

const SERVER_CONFIG_PATH: &str = ".inn_server_config.json";

/// 서버(SSH)에 저장되는 설정
/// 파일 위치: ~/.inn_server_config.json
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ServerConfig {
    #[serde(default)]
    pub cms_config: CmsConfig,
}

impl ServerConfig {
    pub fn load_from_sftp(sftp: &Sftp, home_path: &str) -> Result<Self> {
        let config_path = format!("{}/{}", home_path, SERVER_CONFIG_PATH);
        let path = Path::new(&config_path);

        // 파일이 없으면 기본값 반환
        if sftp.stat(path).is_err() {
            return Ok(ServerConfig::default());
        }

        let mut file = sftp.open(path)
            .context(format!("Failed to open server config: {}", config_path))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .context("Failed to read server config")?;

        let config: ServerConfig = serde_json::from_str(&content)
            .context("Failed to deserialize ServerConfig from JSON")?;

        Ok(config)
    }

    pub fn save_to_sftp(&self, sftp: &Sftp, home_path: &str) -> Result<()> {
        let config_path = format!("{}/{}", home_path, SERVER_CONFIG_PATH);
        let path = Path::new(&config_path);

        let mut file = sftp.create(path)
            .context(format!("Failed to create server config: {}", config_path))?;

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize ServerConfig to JSON")?;

        file.write_all(content.as_bytes())
            .context("Failed to write server config")?;

        Ok(())
    }
}
