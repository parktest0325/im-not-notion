use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use serde_json::Value;

use crate::utils;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SshConfig {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    // #[serde(default)]
    // pub key_path: String,
}

impl SshConfig {
    pub fn load(&mut self, ssh_config: &Value) -> Result<()> {
        *self = serde_json::from_value(ssh_config.clone())
            .context("Failed to deserialize SshConfig from JSON")?;

        // 비밀번호 복호화
        if !self.password.is_empty() {
            self.password = utils::crypto::decrypt_string(&self.password)
                .context("Failed to decrypt SSH password")?;
        }

        Ok(())
    }

    pub fn prepare_for_save(&self) -> Result<Self> {
        let mut config = self.clone();

        // 비밀번호 암호화
        if !config.password.is_empty() {
            config.password = utils::crypto::encrypt_string(&config.password)
                .context("Failed to encrypt SSH password")?;
        }
        Ok(config)
    }
}