//! Agent 控制面板与主服务之间的公开 HTTP 数据契约。

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AgentSettings {
    pub persona: String,
    pub topic: String,
    pub proactive_enabled: bool,
    pub cooldown_ms: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    Chat { text: String },
    Gift { name: String, count: u32 },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum EventPayloadInput {
    Chat { text: String },
    Gift { name: String, count: u32 },
}

impl<'de> Deserialize<'de> for EventPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match EventPayloadInput::deserialize(deserializer)? {
            EventPayloadInput::Chat { text } => Self::Chat { text },
            EventPayloadInput::Gift { name, count } => Self::Gift { name, count },
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum ViewerIdentityKind {
    OpenId,
    Uid,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct ViewerIdentityInput {
    pub namespace: String,
    pub kind: ViewerIdentityKind,
    pub external_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct GiftMetadataInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub price: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub paid: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub medal_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub guard_level: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct LiveEventInput {
    pub id: String,
    pub source: String,
    pub viewer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub viewer_identity: Option<ViewerIdentityInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gift_metadata: Option<GiftMetadataInput>,
    pub kind: EventPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct EventBatchRequest {
    pub events: Vec<LiveEventInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct EventBatchResult {
    pub accepted: u32,
    pub duplicates: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub persisted: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unscheduled: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentPhase {
    Paused,
    Waiting,
    Deciding,
    Speaking,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventStatus {
    Pending,
    Deciding,
    Skipped,
    Expired,
    Queued,
    Synthesizing,
    Ready,
    Playing,
    Completed,
    Cancelled,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct AgentEventSnapshot {
    pub event: LiveEventInput,
    pub status: AgentEventStatus,
    pub speech_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct AgentSnapshot {
    pub paused: bool,
    pub phase: AgentPhase,
    pub settings: AgentSettings,
    pub events: Vec<AgentEventSnapshot>,
    pub last_error: Option<String>,
    pub current_speech_id: Option<String>,
    pub llm_configured: bool,
    pub bridge_connected: bool,
}
