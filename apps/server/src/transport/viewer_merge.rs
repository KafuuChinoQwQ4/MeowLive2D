//! 合并必须先预览并显式确认；与资料和播报版本失效共用写栅栏。
use crate::{state::AppState, transport::error::ApiError};
use axum::{Json, extract::State, http::StatusCode};
use meowlive_application::ports::viewer_merge as port;
use meowlive_protocol::viewer_merge as dto;
fn invalid() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "invalid_viewer_merge",
        "请核对身份、预览版本、合并理由和确认项",
    )
}
fn conflict() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "viewer_merge_conflict",
        "观众资料已变化或合并冲突，请重新预览",
    )
}
fn store(s: &AppState) -> Result<&dyn port::ViewerMergeStore, ApiError> {
    s.viewer_merge_store.as_deref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "merge_unavailable",
            "身份存储当前不可用",
        )
    })
}
fn ids(a: &str, b: &str) -> Result<(), ApiError> {
    if a == b || uuid::Uuid::parse_str(a).is_err() || uuid::Uuid::parse_str(b).is_err() {
        Err(invalid())
    } else {
        Ok(())
    }
}
fn summary(v: port::MergeViewerSummary) -> dto::MergeViewerSummary {
    dto::MergeViewerSummary {
        viewer_id: v.viewer_id,
        alias: v.alias,
        identities: v.identities,
        events: v.events,
        memories: v.memories,
        relationships: v.relationships,
        familiarity_milli: v.familiarity_milli,
        affinity_milli: v.affinity_milli,
    }
}
pub async fn preview(
    State(s): State<AppState>,
    Json(r): Json<dto::ViewerMergePreviewRequest>,
) -> Result<Json<dto::ViewerMergePreview>, ApiError> {
    ids(&r.source_viewer_id, &r.target_viewer_id)?;
    let p = store(&s)?
        .preview_merge(
            &s.config.viewers.scope_id,
            &r.source_viewer_id,
            &r.target_viewer_id,
        )
        .await
        .map_err(|_| conflict())?;
    Ok(Json(dto::ViewerMergePreview {
        source: summary(p.source),
        target: summary(p.target),
        revision: p.revision,
        fingerprint: p.fingerprint,
        resulting_familiarity_milli: p.resulting_familiarity_milli,
        resulting_affinity_milli: p.resulting_affinity_milli,
        risks: p.risks,
    }))
}
pub async fn apply(
    State(s): State<AppState>,
    Json(r): Json<dto::ViewerMergeRequest>,
) -> Result<Json<dto::ViewerMergeOutcome>, ApiError> {
    ids(&r.source_viewer_id, &r.target_viewer_id)?;
    if !r.confirmed
        || r.expected_revision < 0
        || r.fingerprint.len() != 64
        || uuid::Uuid::parse_str(&r.request_key).is_err()
        || r.reason.trim().is_empty()
        || r.reason.len() > 1000
    {
        return Err(invalid());
    }
    let request = port::ViewerMergeRequest {
        source_viewer_id: r.source_viewer_id,
        target_viewer_id: r.target_viewer_id,
        expected_revision: r.expected_revision,
        fingerprint: r.fingerprint,
        request_key: r.request_key,
        reason: r.reason,
        actor: "single_admin".into(),
    };
    let store = store(&s)?;
    let _gate = s.knowledge_gate.write().await;
    s.invalidate_knowledge().await;
    let result = store
        .apply_merge(
            &s.config.viewers.scope_id,
            &request,
            crate::viewers::utc_ms() as i64,
        )
        .await
        .map_err(|_| conflict())?;
    Ok(Json(dto::ViewerMergeOutcome {
        canonical_viewer_id: result.canonical_viewer_id,
        revision: result.revision,
    }))
}
