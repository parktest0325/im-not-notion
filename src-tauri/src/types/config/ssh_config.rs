use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

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
}

impl SshConfig {
    pub fn decrypt_password(&mut self) -> Result<()> {
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