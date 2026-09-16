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
