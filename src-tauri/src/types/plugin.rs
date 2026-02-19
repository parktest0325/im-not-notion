use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/// 서버의 plugin.json 파싱 결과
#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub entry: String,
    pub triggers: Vec<Trigger>,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "content")]
pub enum Trigger {
    #[serde(rename = "manual")]
    Manual {
        label: String,
        input: Vec<InputField>,
    },
    #[serde(rename = "hook")]
    Hook {
        event: HookEvent,
    },
    #[serde(rename = "cron")]
    Cron {
        schedule: String,
        label: String,
    },
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HookEvent {
    AfterFileMove,
    AfterFileSave,
    AfterFileDelete,
    AfterFileCreate,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InputField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub label: String,
    pub default: Option<String>,
}

/// 스크립트 stdout JSON 파싱 결과
#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginResult {
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub actions: Vec<PluginAction>,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "content")]
pub enum PluginAction {
    #[serde(rename = "refresh_tree")]
    RefreshTree,
    #[serde(rename = "toast")]
    Toast { message: String, toast_type: String },
    #[serde(rename = "open_file")]
    OpenFile { path: String },
}
