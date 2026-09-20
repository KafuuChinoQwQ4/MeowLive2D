//! 直播平台连接控制与可公开的诊断快照，不包含授权信息。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum LiveConnectionPhase {
    Disabled,
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Disconnecting,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct LiveConnectionSnapshot {
    pub platform: String,
    pub configured: bool,
    pub phase: LiveConnectionPhase,
    pub room_id: Option<String>,
    pub accepted_events: u32,
    pub duplicate_events: u32,
    pub rejected_events: u32,
    pub reconnect_attempts: u32,
    pub last_error: Option<String>,
}

/// Settings queries expose only presence flags for private credentials.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct LiveSettingsSnapshot {
    pub enabled: bool,
    /// A decimal string preserves application IDs larger than JavaScript's safe integers.
    pub app_id: String,
    pub access_key_id_configured: bool,
    pub access_key_secret_configured: bool,
    pub identity_code_configured: bool,
    pub storage_available: bool,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct LiveSettingsRequest {
    pub enabled: bool,
    pub app_id: String,
    #[serde(default)]
    pub access_key_id: Option<String>,
    #[serde(default)]
    pub access_key_secret: Option<String>,
    #[serde(default)]
    pub identity_code: Option<String>,
    #[serde(default)]
    pub clear_credentials: bool,
}
