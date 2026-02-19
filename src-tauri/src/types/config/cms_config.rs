use serde::{Deserialize, Serialize};
use typeshare::typeshare;

#[typeshare]
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct HugoConfig {
    pub url: String,
    pub hugo_cmd_path: String,
    pub base_path: String,
    pub content_path: String,
    pub image_path: String,
    pub hidden_path: String,
}

impl HugoConfig {
    pub fn is_empty(&self) -> bool {
        self.base_path.is_empty() && self.content_path.is_empty()
    }

    /// 일반 콘텐츠 절대경로: {base_path}/content/{content_path}{suffix}
    pub fn content_abs(&self, suffix: &str) -> String {
        format!("{}/content/{}{}", self.base_path, self.content_path, suffix)
    }

    /// 숨김 콘텐츠 절대경로: {base_path}/content/{hidden_path}/{content_path}{suffix}
    pub fn hidden_abs(&self, suffix: &str) -> String {
        format!("{}/content/{}/{}{}", self.base_path, self.hidden_path, self.content_path, suffix)
    }
}

#[typeshare]
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct CmsConfig {
    pub hugo_config: HugoConfig,
}