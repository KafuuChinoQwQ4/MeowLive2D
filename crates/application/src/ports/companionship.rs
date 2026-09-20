//! 管理员专属陪伴事实及积分流水；不进入模型上下文。
use super::viewers::ViewerStoreFuture;
use meowlive_domain::event::{GiftMetadata, LiveEvent};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffinityAdjustment {
    pub request_key: String,
    pub reason: String,
    pub delta_milli: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffinityReversal {
    pub request_key: String,
    pub reason: String,
    pub ledger_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GiftConfirmation {
    pub request_key: String,
    pub reason: String,
    pub source: String,
    pub event_id: String,
    pub value_cents: u64,
    pub value_kind: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffinityLedgerEntry {
    pub reversible: bool,
    pub ledger_id: String,
    pub kind: String,
    pub computed_delta_milli: i32,
    pub applied_delta_milli: i32,
    pub reason: String,
    pub actor: String,
    pub created_at_ms: u64,
    pub reversed_ledger_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GiftLedgerEntry {
    pub source: String,
    pub event_id: String,
    pub name: String,
    pub count: u32,
    pub metadata: Option<GiftMetadata>,
    pub value_cents: Option<u64>,
    pub value_kind: String,
    pub occurred_at_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanionshipDetail {
    pub viewer_id: String,
    pub familiarity_milli: i32,
    pub affinity_milli: i32,
    pub observed_days: u64,
    pub observed_sessions: u64,
    pub last_seen_at_ms: u64,
    pub medal_level: Option<u32>,
    pub guard_level: Option<u32>,
    pub gifts: Vec<GiftLedgerEntry>,
    pub ledger: Vec<AffinityLedgerEntry>,
}
pub trait CompanionshipStore: Send + Sync {
    fn detail<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        limit: u32,
    ) -> ViewerStoreFuture<'a, Option<CompanionshipDetail>>;
    fn register_reply<'a>(
        &'a self,
        scope: &'a str,
        speech_id: &'a str,
        events: &'a [LiveEvent],
        now: u64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn complete_reply<'a>(
        &'a self,
        scope: &'a str,
        speech_id: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn adjust<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        request: &'a AffinityAdjustment,
        now: u64,
    ) -> ViewerStoreFuture<'a, String>;
    fn reverse<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        request: &'a AffinityReversal,
        now: u64,
    ) -> ViewerStoreFuture<'a, String>;
    fn confirm_gift<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        request: &'a GiftConfirmation,
        now: u64,
    ) -> ViewerStoreFuture<'a, String>;
}
