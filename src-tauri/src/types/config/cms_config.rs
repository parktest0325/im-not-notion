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

    /// 일반 콘텐츠 절대경로: {base_path}/content/{content_path}{suffix}
    pub fn content_abs(&self, suffix: &str) -> String {
        format!("{}/content/{}{}", self.base_path, self.content_path, suffix)
    }

    /// 숨김 콘텐츠 절대경로: {base_path}/content/{hidden_path}/{content_path}{suffix}
    pub fn hidden_abs(&self, suffix: &str) -> String {
        format!("{}/content/{}/{}{}", self.base_path, self.hidden_path, self.content_path, suffix)
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct CmsConfig {
    #[serde(default)]
    pub hugo_config: HugoConfig,
}