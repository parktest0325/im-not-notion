use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use dirs_next::home_dir;
use anyhow::{Result, Context};

use super::SshConfig;

/// 클라이언트(로컬)에 저장되는 설정
/// 파일 위치: ~/.inn_config.json
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ClientConfig {
    #[serde(default)]
    pub ssh_config: SshConfig,
}

impl ClientConfig {
    pub fn get_config_path() -> PathBuf {
        home_dir().unwrap().join(".inn_config.json")
    }

    pub fn load_from_file() -> Result<Self> {
        let config_file_path = Self::get_config_path();
        let file = File::open(&config_file_path)
            .context(format!("Failed to open config file: {:?}", config_file_path))?;
        let mut config: ClientConfig = serde_json::from_reader(file)
            .context("Failed to deserialize ClientConfig from JSON")?;

        config.ssh_config.decrypt_password()?;
        Ok(config)
    }

    pub fn save_to_file(&self) -> Result<()> {
        let config_file_path = Self::get_config_path();
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&config_file_path)
            .context(format!("Failed to create config file: {:?}", config_file_path))?;

        let save_config = ClientConfig {
            ssh_config: self.ssh_config.prepare_for_save()?,
        };

        serde_json::to_writer_pretty(file, &save_config)?;
        Ok(())
    }
}
