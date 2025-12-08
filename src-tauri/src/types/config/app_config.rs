use serde::{Deserialize, Serialize};
use anyhow::Result;
use ssh2::Sftp;

use super::{ClientConfig, CmsConfig, ServerConfig, SshConfig};

/// 프론트엔드와 통신하는 통합 설정 구조체
/// 실제 저장은 ClientConfig(로컬)와 ServerConfig(서버)로 분리됨
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub ssh_config: SshConfig,
    #[serde(default)]
    pub cms_config: CmsConfig,
}

impl AppConfig {
    /// SSH 설정이 있는지 확인
    pub fn has_ssh_config(&self) -> bool {
        !self.ssh_config.host.is_empty() && !self.ssh_config.username.is_empty()
    }

    /// AppConfig를 ClientConfig로 분리
    pub fn to_client_config(&self) -> ClientConfig {
        ClientConfig {
            ssh_config: self.ssh_config.clone(),
        }
    }

    /// AppConfig를 ServerConfig로 분리
    pub fn to_server_config(&self) -> ServerConfig {
        ServerConfig {
            cms_config: self.cms_config.clone(),
        }
    }

    /// 로컬 설정 파일에서 ClientConfig만 로드
    pub fn load_client_only() -> Result<Self> {
        let client = ClientConfig::load_from_file()?;
        Ok(AppConfig {
            ssh_config: client.ssh_config,
            cms_config: CmsConfig::default(),
        })
    }

    /// 서버에서 ServerConfig 로드하여 합침
    pub fn load_server_config(&mut self, sftp: &Sftp, home_path: &str) -> Result<()> {
        let server = ServerConfig::load_from_sftp(sftp, home_path)?;
        self.cms_config = server.cms_config;
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