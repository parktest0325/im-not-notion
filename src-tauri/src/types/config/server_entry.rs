use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use super::SshConfig;

/// 서버 항목: ID + 이름 + SSH 설정
#[typeshare]
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerEntry {
    pub id: String,
    pub name: String,
    pub ssh_config: SshConfig,
}
