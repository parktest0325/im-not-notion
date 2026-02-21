use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use dirs_next::home_dir;
use anyhow::{Result, Context};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use super::{SshConfig, ServerEntry};

/// 클라이언트(로컬)에 저장되는 설정
/// 파일 위치: ~/.inn_config.json
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ClientConfig {
    #[serde(default)]
    pub active_server: String,
    #[serde(default)]
    pub servers: Vec<ServerEntry>,
    #[serde(default)]
    pub plugin_local_path: String,

    // 하위호환: 기존 단일 ssh_config → 마이그레이션용 (저장 시 제외)
    #[serde(default, skip_serializing)]
    ssh_config: SshConfig,
}

impl ClientConfig {
    pub fn new(active_server: String, servers: Vec<ServerEntry>, plugin_local_path: String) -> Self {
        ClientConfig {
            active_server,
            servers,
            plugin_local_path,
            ssh_config: SshConfig::default(),
        }
    }

    pub fn get_config_path() -> Result<PathBuf> {
        let home = home_dir().context("Failed to determine home directory")?;
        Ok(home.join(".inn_config.json"))
    }

    pub fn load_from_file() -> Result<Self> {
        let config_file_path = Self::get_config_path()?;
        let file = File::open(&config_file_path)
            .context(format!("Failed to open config file: {:?}", config_file_path))?;
        let mut config: ClientConfig = serde_json::from_reader(file)
            .context("Failed to deserialize ClientConfig from JSON")?;

        // 마이그레이션: 기존 단일 ssh_config → servers 배열로 변환
        if config.servers.is_empty() && !config.ssh_config.host.is_empty() {
            config.ssh_config.decrypt_password()?;
            let id: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();
            let entry = ServerEntry {
                id: id.clone(),
                name: config.ssh_config.host.clone(),
                ssh_config: config.ssh_config.clone(),
            };
            config.servers.push(entry);
            config.active_server = id;
            config.ssh_config = SshConfig::default();
        } else {
            // 각 서버의 비밀번호 복호화
            for server in &mut config.servers {
                server.ssh_config.decrypt_password()?;
            }
        }

        Ok(config)
    }

    pub fn save_to_file(&self) -> Result<()> {
        let config_file_path = Self::get_config_path()?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&config_file_path)
            .context(format!("Failed to create config file: {:?}", config_file_path))?;

        let save_config = ClientConfig {
            active_server: self.active_server.clone(),
            servers: self.servers.iter().map(|s| {
                ServerEntry {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    ssh_config: s.ssh_config.prepare_for_save().unwrap_or_default(),
                }
            }).collect(),
            plugin_local_path: self.plugin_local_path.clone(),
            ssh_config: SshConfig::default(),
        };

        serde_json::to_writer_pretty(file, &save_config)?;
        Ok(())
    }

    /// active_server에 해당하는 ServerEntry의 SshConfig 반환
    pub fn get_active_ssh_config(&self) -> Option<SshConfig> {
        self.servers.iter()
            .find(|s| s.id == self.active_server)
            .map(|s| s.ssh_config.clone())
    }
}
