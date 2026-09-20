//! 权威记忆与有租约后台任务；所有调用必须提供隔离范围。
use super::{memory::EmbeddingBatch, viewers::ViewerStoreFuture};
use meowlive_domain::memory::{MemoryCandidate, MemorySource, MemoryStatus};
#[derive(Clone, Debug, PartialEq)]
pub struct MemoryRecord {
    pub id: String,
    pub viewer_id: String,
    pub candidate: MemoryCandidate,
    pub status: MemoryStatus,
    pub version: i64,
    pub locked: bool,
    pub deleted: bool,
    pub expires_at_ms: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractionJob {
    pub id: String,
    pub token: String,
    pub revision: i64,
    pub attempts: i32,
    pub sources: Vec<MemorySource>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MemorySnapshot {
    pub revision: i64,
    pub records: Vec<MemoryRecord>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct QueryEmbedding {
    pub model: String,
    pub vector: Vec<f32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddingJob {
    pub memory_id: String,
    pub version: i64,
    pub token: String,
    pub text: String,
    pub model: String,
    pub dimensions: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAdminRequest {
    pub viewer_id: String,
    pub memory_id: String,
    pub expected_version: i64,
    pub request_key: String,
    pub reason: String,
    pub actor: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryQueueStatus {
    pub pending: i64,
    pub running: i64,
    pub failed: i64,
    pub embedding_pending: i64,
    pub embedding_failed: i64,
}
pub trait MemoryStore: Send + Sync {
    fn retry_failed<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryMaintenanceRequest,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn rebuild_vectors<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryMaintenanceRequest,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()>;

    fn resolve_viewers<'a>(
        &'a self,
        scope: &'a str,
        events: &'a [meowlive_domain::event::LiveEvent],
    ) -> ViewerStoreFuture<'a, Vec<String>>;
    fn claim_job<'a>(
        &'a self,
        scope: &'a str,
        now: i64,
    ) -> ViewerStoreFuture<'a, Option<ExtractionJob>>;
    fn finish_job<'a>(
        &'a self,
        scope: &'a str,
        job: &'a ExtractionJob,
        candidates: &'a [MemoryCandidate],
        now: i64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn fail_job<'a>(
        &'a self,
        scope: &'a str,
        job: &'a ExtractionJob,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn list<'a>(
        &'a self,
        scope: &'a str,
        viewer_id: &'a str,
        limit: u32,
        now: i64,
    ) -> ViewerStoreFuture<'a, Vec<MemoryRecord>>;
    fn context<'a>(
        &'a self,
        scope: &'a str,
        viewer_ids: &'a [String],
        query: Option<&'a QueryEmbedding>,
        now: i64,
    ) -> ViewerStoreFuture<'a, MemorySnapshot>;
    fn revision<'a>(&'a self, scope: &'a str) -> ViewerStoreFuture<'a, i64>;
    fn admin_correct<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryAdminRequest,
        value: &'a str,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn admin_delete<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryAdminRequest,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn admin_freeze<'a>(
        &'a self,
        scope: &'a str,
        request: &'a MemoryAdminRequest,
        frozen: bool,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn claim_embedding<'a>(
        &'a self,
        scope: &'a str,
        model: &'a str,
        dimensions: usize,
        now: i64,
    ) -> ViewerStoreFuture<'a, Option<EmbeddingJob>>;
    fn store_embedding<'a>(
        &'a self,
        scope: &'a str,
        job: &'a EmbeddingJob,
        batch: &'a EmbeddingBatch,
        now: i64,
    ) -> ViewerStoreFuture<'a, ()>;
    fn status<'a>(&'a self, scope: &'a str) -> ViewerStoreFuture<'a, MemoryQueueStatus>;
    fn maintenance<'a>(&'a self, scope: &'a str, now: i64) -> ViewerStoreFuture<'a, ()>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryMaintenanceRequest {
    pub request_key: String,
    pub reason: String,
    pub actor: String,
}
