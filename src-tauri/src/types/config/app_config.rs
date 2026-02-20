use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use ssh2::Sftp;
use typeshare::typeshare;

use super::{ClientConfig, CmsConfig, ServerConfig, ServerEntry, SshConfig};


/// 프론트엔드와 통신하는 통합 설정 구조체
/// 실제 저장은 ClientConfig(로컬)와 ServerConfig(서버)로 분리됨
#[typeshare]
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub active_server: String,
    #[serde(default)]
    pub servers: Vec<ServerEntry>,
    pub cms_config: CmsConfig,
    #[serde(default)]
    pub shortcuts: HashMap<String, Vec<String>>,
}

impl AppConfig {
    /// active_server에 해당하는 SSH 설정 반환
    pub fn get_active_ssh_config(&self) -> Option<SshConfig> {
        self.servers.iter()
            .find(|s| s.id == self.active_server)
            .map(|s| s.ssh_config.clone())
    }

    /// AppConfig를 ClientConfig로 분리
    pub fn to_client_config(&self) -> ClientConfig {
        ClientConfig::new(self.active_server.clone(), self.servers.clone())
    }

    /// AppConfig를 ServerConfig로 분리
    pub fn to_server_config(&self) -> ServerConfig {
        ServerConfig {
            cms_config: self.cms_config.clone(),
            shortcuts: self.shortcuts.clone(),
        }
    }

    /// 서버에서 ServerConfig 로드하여 합침
    pub fn load_server_config(&mut self, sftp: &Sftp, home_path: &str) -> Result<()> {
        let server = ServerConfig::load_from_sftp(sftp, home_path)?;
        self.cms_config = server.cms_config;
        self.shortcuts = server.shortcuts;
        Ok(())
    }

    /// ClientConfig를 로컬에 저장
    pub fn save_client_config(&self) -> Result<()> {
        self.to_client_config().save_to_file()
    }

    /// ServerConfig를 서버에 저장
    pub fn save_server_config(&self, sftp: &Sftp, home_path: &str) -> Result<()> {
        self.to_server_config().save_to_sftp(sftp, home_path)
    }
}
