//! LLM 接入配置契约；查询只返回密钥是否存在，不返回密钥内容。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct LlmSettings {
    pub provider: String,
    pub api_format: String,
    pub base_url: String,
    pub model: String,
    pub mode: String,
    pub timeout_seconds: u32,
    pub max_tokens: u32,
    pub json_mode: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct LlmSettingsSnapshot {
    pub settings: LlmSettings,
    pub key_configured: bool,
    pub restart_required: bool,
    pub active_model: String,
    pub storage_available: bool,
}

#[derive(Clone, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct LlmSettingsRequest {
    pub settings: LlmSettings,
    // None preserves the saved key only for the same provider, format and base URL.
    pub api_key: Option<String>,
    pub clear_api_key: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct LlmTestResult {
    pub message: String,
}
