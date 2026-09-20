//! 身份合并预览与显式确认，不使用昵称自动绑定。
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct MergeViewerSummary {
    pub viewer_id: String,
    pub alias: Option<String>,
    #[ts(type = "number")]
    pub identities: i64,
    #[ts(type = "number")]
    pub events: i64,
    #[ts(type = "number")]
    pub memories: i64,
    #[ts(type = "number")]
    pub relationships: i64,
    pub familiarity_milli: i32,
    pub affinity_milli: i32,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ViewerMergePreview {
    pub source: MergeViewerSummary,
    pub target: MergeViewerSummary,
    #[ts(type = "number")]
    pub revision: i64,
    pub fingerprint: String,
    pub resulting_familiarity_milli: i32,
    pub resulting_affinity_milli: i32,
    pub risks: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ViewerMergePreviewRequest {
    pub source_viewer_id: String,
    pub target_viewer_id: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ViewerMergeRequest {
    pub source_viewer_id: String,
    pub target_viewer_id: String,
    #[ts(type = "number")]
    pub expected_revision: i64,
    pub fingerprint: String,
    pub request_key: String,
    pub reason: String,
    pub confirmed: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ViewerMergeOutcome {
    pub canonical_viewer_id: String,
    #[ts(type = "number")]
    pub revision: i64,
}
