use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct HugoConfig {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub hugo_cmd_path: String,
    #[serde(default)]
    pub base_path: String,
    #[serde(default)]
    pub content_path: String,
    #[serde(default)]
    pub image_path: String,
    #[serde(default)]
    pub hidden_path: String,
}

impl HugoConfig {
    pub fn is_empty(&self) -> bool {
        self.base_path.is_empty() && self.content_path.is_empty()
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct CmsConfig {
    #[serde(default)]
    pub hugo_config: HugoConfig,
}