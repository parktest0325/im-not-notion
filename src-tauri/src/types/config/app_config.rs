use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use tauri::api::path::home_dir;
use anyhow::Result;
use serde_json::Value;

use super::{CmsConfig, SshConfig};


#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub ssh_config: SshConfig,
    #[serde(default)]
    pub cms_config: CmsConfig,
}

impl AppConfig {
    fn get_config_path() -> PathBuf {
        home_dir().unwrap().join(".inn_config.json")
    }

    pub fn load_config_from_file(&mut self) -> Result<()> {
        let config_file_path = Self::get_config_path();
        let file = File::open(config_file_path)?;
        let config_data: Value = serde_json::from_reader(file)?;

        self.ssh_config.load(&config_data["ssh_config"])?;
        self.cms_config.load(&config_data["cms_config"])?;

        Ok(())
    }

    // config를 파일에 저장
    pub fn save_config_to_file(&self) -> Result<()> {
        let config_file_path = Self::get_config_path();
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_file_path)?;

        let save_config = AppConfig {
            ssh_config: self.ssh_config.prepare_for_save()?,
            cms_config: self.cms_config.prepare_for_save()?,
        };

        serde_json::to_writer_pretty(file, &save_config)?;

        Ok(())
    }
}