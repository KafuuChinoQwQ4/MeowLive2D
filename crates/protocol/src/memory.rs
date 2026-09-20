//! 管理员记忆详情与有版本条件的操作，禁止作为公开直播契约发送。
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MemoryEvidence {
    pub source: String,
    pub event_id: String,
    pub quote: String,
    #[ts(type = "number")]
    pub occurred_at_ms: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ViewerMemory {
    pub id: String,
    pub viewer_id: String,
    pub key: String,
    pub value: String,
    pub kind: String,
    pub status: String,
    #[ts(type = "number")]
    pub version: i64,
    pub locked: bool,
    pub deleted: bool,
    #[ts(type = "number | null")]
    pub expires_at_ms: Option<i64>,
    pub evidence: Vec<MemoryEvidence>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MemoryPage {
    pub memories: Vec<ViewerMemory>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct MemoryMutationRequest {
    pub request_key: String,
    pub reason: String,
    #[ts(type = "number")]
    pub expected_version: i64,
    pub operation: String,
    pub value: Option<String>,
    pub frozen: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MemoryJobsStatus {
    #[ts(type = "number")]
    pub pending: i64,
    #[ts(type = "number")]
    pub running: i64,
    #[ts(type = "number")]
    pub failed: i64,
    #[ts(type = "number")]
    pub embedding_pending: i64,
    #[ts(type = "number")]
    pub embedding_failed: i64,
}
