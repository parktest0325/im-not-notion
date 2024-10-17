use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use serde_json::Value;

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
    pub config_path: String,
    #[serde(default)]
    pub layout_path: String,
    #[serde(default)]
    pub trashcan_path: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct CmsConfig {
    #[serde(default)]
    pub hugo_config: HugoConfig,
    // 향후 추가될 CMS 설정들도 여기에 포함될 수 있음
}

impl CmsConfig {
    pub fn load(&mut self, cms_config: &Value) -> Result<()> {
        *self = serde_json::from_value(cms_config.clone())
            .context("Failed to deserialize CmsConfig from JSON")?;
        Ok(())
    }

    pub fn prepare_for_save(&self) -> Result<CmsConfig> {
        // 현재는 특별한 전처리가 필요 없지만, 향후 필요한 경우 여기에 구현
        Ok(self.clone())
    }
}