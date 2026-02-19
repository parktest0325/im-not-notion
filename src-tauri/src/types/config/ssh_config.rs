use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use typeshare::typeshare;

use crate::utils;

#[typeshare]
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SshConfig {
    pub host: String,
    pub port: String,
    pub username: String,
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