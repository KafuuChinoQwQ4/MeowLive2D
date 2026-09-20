//! 仅供已认证管理员的关系状态、贡献依据和调整请求。
use crate::agent::GiftMetadataInput;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AffinityLedgerEntry {
    pub ledger_id: String,
    pub kind: String,
    pub computed_delta_milli: i32,
    pub applied_delta_milli: i32,
    pub reason: String,
    pub actor: String,
    #[ts(type = "number")]
    pub created_at_ms: u64,
    pub reversed_ledger_id: Option<String>,
    pub reversible: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct GiftLedgerEntry {
    pub source: String,
    pub event_id: String,
    pub name: String,
    pub count: u32,
    pub metadata: Option<GiftMetadataInput>,
    #[ts(type = "number | null")]
    pub value_cents: Option<u64>,
    pub value_kind: String,
    #[ts(type = "number")]
    pub occurred_at_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct CompanionshipDetail {
    pub viewer_id: String,
    pub familiarity_milli: i32,
    pub affinity_milli: i32,
    #[ts(type = "number")]
    pub observed_days: u64,
    #[ts(type = "number")]
    pub observed_sessions: u64,
    #[ts(type = "number")]
    pub last_seen_at_ms: u64,
    pub medal_level: Option<u32>,
    pub guard_level: Option<u32>,
    pub gifts: Vec<GiftLedgerEntry>,
    pub ledger: Vec<AffinityLedgerEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AffinityAdjustmentRequest {
    pub request_key: String,
    pub reason: String,
    pub delta_milli: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AffinityReversalRequest {
    pub request_key: String,
    pub reason: String,
    pub ledger_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct GiftConfirmationRequest {
    pub request_key: String,
    pub reason: String,
    pub source: String,
    pub event_id: String,
    #[ts(type = "number")]
    pub value_cents: u64,
    pub value_kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct AdminMutationResult {
    pub record_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ReceiptProblem {
    pub speech_id: String,
    pub attempts: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct CompanionshipHealth {
    pub durable_receipts: bool,
    pub failed_receipts: Vec<ReceiptProblem>,
    pub pending_receipts: u32,
    #[ts(type = "number")]
    pub failed_receipt_attempts: u64,
}
