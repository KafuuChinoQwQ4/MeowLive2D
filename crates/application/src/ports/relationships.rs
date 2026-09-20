//! SQL权威关系事实、持久图同步及可替换图查询能力。
use super::viewers::ViewerStoreFuture;
pub use meowlive_domain::relationships::*;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationInput {
    pub source: RelationEntity,
    pub target: RelationEntity,
    pub kind: RelationKind,
    pub evidence: Vec<RelationEvidence>,
    pub expires_at_ms: Option<u64>,
    pub admin_confirmed: bool,
    pub reason: String,
    pub request_key: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationAction {
    Confirm,
    Revoke,
    Delete,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphReference {
    pub fact_id: String,
    pub version: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphProjection {
    pub scope: String,
    pub fact: RelationshipFact,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphOutboxLease {
    pub id: String,
    pub token: String,
    pub projection: GraphProjection,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphSyncStatus {
    pub pending: u64,
    pub leased: u64,
    pub failed: u64,
    pub oldest_pending_age_ms: Option<u64>,
}
pub trait RelationshipStore: Send + Sync {
    fn create<'a>(
        &'a self,
        scope: &'a str,
        input: &'a RelationInput,
        now: u64,
    ) -> ViewerStoreFuture<'a, RelationshipFact>;
    #[allow(clippy::too_many_arguments)]
    fn change<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        expected_version: u64,
        action: RelationAction,
        reason: &'a str,
        request_key: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, RelationshipFact>;
    /// Administrator view includes claims, expired facts and deletion tombstones.
    fn list_admin<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        limit: u32,
        offset: u64,
    ) -> ViewerStoreFuture<'a, Vec<RelationshipFact>>;
    /// Active retrieval expires_at_ms includes the earliest source-memory deadline.
    fn query<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        depth: u8,
        limit: u32,
        now: u64,
    ) -> ViewerStoreFuture<'a, Vec<RelationshipFact>>;
    /// Revalidates versions and returns the same effective provenance deadline as query.
    fn validate_graph<'a>(
        &'a self,
        scope: &'a str,
        refs: &'a [GraphReference],
        now: u64,
    ) -> ViewerStoreFuture<'a, Vec<RelationshipFact>>;
    fn claim_outbox<'a>(
        &'a self,
        scope: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, Option<GraphOutboxLease>>;
    fn finish_outbox<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        token: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn fail_outbox<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        token: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn status<'a>(&'a self, scope: &'a str, now: u64) -> ViewerStoreFuture<'a, GraphSyncStatus>;
    fn rebuild<'a>(
        &'a self,
        scope: &'a str,
        request_key: &'a str,
        reason: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, u64>;
}
pub trait RelationshipGraph: Send + Sync {
    fn project<'a>(&'a self, snapshot: &'a GraphProjection) -> ViewerStoreFuture<'a, ()>;
    fn neighbors<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        depth: u8,
        limit: u32,
    ) -> ViewerStoreFuture<'a, Vec<GraphReference>>;
}
