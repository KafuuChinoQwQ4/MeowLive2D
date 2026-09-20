//! 受保护关系事实、证据和图同步状态契约。
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RelationshipEntity {
    pub kind: String,
    pub id: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RelationshipEvidence {
    pub source: String,
    pub event_id: String,
    pub quote: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ViewerRelationship {
    pub id: String,
    #[ts(type = "number")]
    pub version: u64,
    pub source: RelationshipEntity,
    pub target: RelationshipEntity,
    pub kind: String,
    pub confirmation: String,
    pub evidence: Vec<RelationshipEvidence>,
    #[ts(type = "number | null")]
    pub expires_at_ms: Option<u64>,
    pub deleted: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct RelationshipPage {
    pub relationships: Vec<ViewerRelationship>,
    pub degraded: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RelationshipCreateRequest {
    pub source: RelationshipEntity,
    pub target: RelationshipEntity,
    pub kind: String,
    pub evidence: Vec<RelationshipEvidence>,
    #[ts(type = "number | null")]
    pub expires_at_ms: Option<u64>,
    pub admin_confirmed: bool,
    pub reason: String,
    pub request_key: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RelationshipChangeRequest {
    #[ts(type = "number")]
    pub expected_version: u64,
    pub action: String,
    pub reason: String,
    pub request_key: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeMaintenanceRequest {
    pub request_key: String,
    pub reason: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct GraphStatus {
    #[ts(type = "number")]
    pub pending: u64,
    #[ts(type = "number")]
    pub leased: u64,
    #[ts(type = "number")]
    pub failed: u64,
    #[ts(type = "number | null")]
    pub oldest_pending_age_ms: Option<u64>,
    pub connected: bool,
}
