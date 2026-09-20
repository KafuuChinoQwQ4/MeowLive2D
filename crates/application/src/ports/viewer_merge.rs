//! 管理员显式身份合并：可核对预览、版本指纹和审计化应用。
use super::viewers::ViewerStoreFuture;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeViewerSummary {
    pub viewer_id: String,
    pub alias: Option<String>,
    pub identities: i64,
    pub events: i64,
    pub memories: i64,
    pub relationships: i64,
    pub familiarity_milli: i32,
    pub affinity_milli: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerMergePreview {
    pub source: MergeViewerSummary,
    pub target: MergeViewerSummary,
    pub revision: i64,
    pub fingerprint: String,
    pub resulting_familiarity_milli: i32,
    pub resulting_affinity_milli: i32,
    pub risks: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerMergeRequest {
    pub source_viewer_id: String,
    pub target_viewer_id: String,
    pub expected_revision: i64,
    pub fingerprint: String,
    pub request_key: String,
    pub reason: String,
    pub actor: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerMergeOutcome {
    pub canonical_viewer_id: String,
    pub revision: i64,
}
pub trait ViewerMergeStore: Send + Sync {
    fn preview_merge<'a>(
        &'a self,
        scope: &'a str,
        source: &'a str,
        target: &'a str,
    ) -> ViewerStoreFuture<'a, ViewerMergePreview>;
    fn apply_merge<'a>(
        &'a self,
        scope: &'a str,
        request: &'a ViewerMergeRequest,
        now: i64,
    ) -> ViewerStoreFuture<'a, ViewerMergeOutcome>;
}
