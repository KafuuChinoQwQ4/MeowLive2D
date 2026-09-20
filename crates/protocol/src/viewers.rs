//! 管理员观众与事件摘要；不用于观众公开接口或执行通道。
use crate::agent::{EventPayload, GiftMetadataInput};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ViewerAlias {
    pub alias: String,
    #[ts(type = "number")]
    pub first_seen_at_ms: u64,
    #[ts(type = "number")]
    pub last_seen_at_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ViewerIdentity {
    pub platform: String,
    pub namespace: String,
    pub id_kind: String,
    pub external_id: String,
    #[ts(type = "number")]
    pub last_confirmed_at_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ViewerSummary {
    pub viewer_id: String,
    pub current_alias: Option<String>,
    #[ts(type = "number | null")]
    pub alias_observed_at_ms: Option<u64>,
    pub identities: Vec<ViewerIdentity>,
    pub aliases: Vec<ViewerAlias>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct PersistedViewerEvent {
    pub event_id: String,
    pub source: String,
    pub session_id: String,
    pub viewer_id: Option<String>,
    pub viewer: String,
    #[ts(type = "number")]
    pub occurred_at_ms: u64,
    #[ts(type = "number")]
    pub received_at_ms: u64,
    pub kind: EventPayload,
    pub gift_metadata: Option<GiftMetadataInput>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ViewerPage {
    pub scope_id: String,
    pub viewers: Vec<ViewerSummary>,
    #[ts(type = "number")]
    pub offset: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ViewerEventPage {
    pub scope_id: String,
    pub events: Vec<PersistedViewerEvent>,
    #[ts(type = "number")]
    pub offset: u64,
    /// Since this server process started; absence of a counter is not proof of complete upstream data.
    #[ts(type = "number")]
    pub unconfirmed_events: u64,
}
