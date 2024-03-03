use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, ErrorKind};
use std::path::Path;

pub const SETTING_FILE_PATH: &str = "./cms_config.json";

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SshClientConfig {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub key_path: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct AppConfig {
    pub ssh_client: SshClientConfig,
}

pub fn save_config(config: &AppConfig, path: &Path) -> io::Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, config)?;
    Ok(())
}

pub fn load_config(path: &Path) -> io::Result<AppConfig> {
    let file = File::open(path)?;
    let config = serde_json::from_reader(file)?;
    Ok(config)
}
