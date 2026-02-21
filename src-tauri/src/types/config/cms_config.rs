use serde::{Serialize, Deserialize, Deserializer};
use typeshare::typeshare;

#[typeshare]
#[derive(Serialize, Debug, Clone)]
pub struct HugoConfig {
    pub url: String,
    pub hugo_cmd_path: String,
    pub base_path: String,
    pub content_paths: Vec<String>,
    pub image_path: String,
    pub hidden_path: String,
}

impl Default for HugoConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            hugo_cmd_path: String::new(),
            base_path: String::new(),
            content_paths: Vec::new(),
            image_path: String::new(),
            hidden_path: String::new(),
        }
    }
}

/// 역직렬화: content_path (문자열) / content_paths (배열) 둘 다 지원
impl<'de> Deserialize<'de> for HugoConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            url: String,
            #[serde(default)]
            hugo_cmd_path: String,
            #[serde(default)]
            base_path: String,
            #[serde(default)]
            content_path: Option<String>,
            #[serde(default)]
            content_paths: Option<Vec<String>>,
            #[serde(default)]
            image_path: String,
            #[serde(default)]
            hidden_path: String,
        }

        let raw = Raw::deserialize(deserializer)?;

        let content_paths = if let Some(paths) = raw.content_paths {
            paths.into_iter().filter(|s| !s.is_empty()).collect()
        } else if let Some(path) = raw.content_path {
            if path.is_empty() { vec![] } else { vec![path] }
        } else {
            vec![]
        };

        Ok(HugoConfig {
            url: raw.url,
            hugo_cmd_path: raw.hugo_cmd_path,
            base_path: raw.base_path,
            content_paths,
            image_path: raw.image_path,
            hidden_path: raw.hidden_path,
        })
    }
}

impl HugoConfig {
    pub fn is_empty(&self) -> bool {
        self.base_path.is_empty() && self.content_paths.is_empty()
    }

    /// 일반 콘텐츠 절대경로: {base_path}/content{suffix}
    /// suffix에 섹션이 포함됨: e.g. "/posts/my-post/_index.md"
    pub fn content_abs(&self, suffix: &str) -> String {
        format!("{}/content{}", self.base_path, suffix)
    }

    /// 숨김 콘텐츠 절대경로: {base_path}/content/{hidden_path}{suffix}
    pub fn hidden_abs(&self, suffix: &str) -> String {
        format!("{}/content/{}{}", self.base_path, self.hidden_path, suffix)
    }
}

#[typeshare]
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct CmsConfig {
    pub hugo_config: HugoConfig,
}
