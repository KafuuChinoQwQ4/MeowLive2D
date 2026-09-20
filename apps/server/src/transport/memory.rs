//! 受保护记忆查询及修正/删除/冻结；版本条件防止覆盖管理员的较新操作。
use crate::{state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use meowlive_application::ports::memory_store::{MemoryAdminRequest, MemoryStore};
use meowlive_protocol::memory as dto;
fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "memory_unavailable",
        "记忆存储当前不可用",
    )
}
fn invalid() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "invalid_memory_operation",
        "记忆操作的标识、版本或依据无效",
    )
}
fn store(s: &AppState) -> Result<&dyn MemoryStore, ApiError> {
    s.memory_store.as_deref().ok_or_else(unavailable)
}
fn id(s: &str) -> Result<(), ApiError> {
    uuid::Uuid::parse_str(s).map(|_| ()).map_err(|_| invalid())
}
pub async fn list(
    State(s): State<AppState>,
    Path(viewer): Path<String>,
) -> Result<Json<dto::MemoryPage>, ApiError> {
    id(&viewer)?;
    let records = store(&s)?
        .list(
            &s.config.viewers.scope_id,
            &viewer,
            250,
            crate::viewers::utc_ms() as i64,
        )
        .await
        .map_err(|_| unavailable())?;
    Ok(Json(dto::MemoryPage {
        memories: records
            .into_iter()
            .map(|r| dto::ViewerMemory {
                id: r.id,
                viewer_id: r.viewer_id,
                key: r.candidate.key,
                value: r.candidate.value,
                kind: format!("{:?}", r.candidate.kind),
                status: format!("{:?}", r.status),
                version: r.version,
                locked: r.locked,
                deleted: r.deleted,
                expires_at_ms: r.expires_at_ms,
                evidence: r
                    .candidate
                    .evidence
                    .into_iter()
                    .map(|e| dto::MemoryEvidence {
                        source: e.source,
                        event_id: e.event_id,
                        quote: e.quote,
                        occurred_at_ms: e.occurred_at_ms,
                    })
                    .collect(),
            })
            .collect(),
    }))
}
pub async fn mutate(
    State(s): State<AppState>,
    Path((viewer, memory)): Path<(String, String)>,
    Json(req): Json<dto::MemoryMutationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    id(&viewer)?;
    id(&memory)?;
    id(&req.request_key)?;
    if req.expected_version < 1 || req.reason.trim().is_empty() || req.reason.len() > 1000 {
        return Err(invalid());
    }
    match req.operation.as_str() {
        "correct"
            if req
                .value
                .as_ref()
                .is_some_and(|v| !v.trim().is_empty() && v.len() <= 512) => {}
        "delete" => {}
        "freeze" if req.frozen.is_some() => {}
        _ => return Err(invalid()),
    }
    let request = MemoryAdminRequest {
        viewer_id: viewer,
        memory_id: memory,
        expected_version: req.expected_version,
        request_key: req.request_key,
        reason: req.reason,
        actor: "single_admin".into(),
    };
    let _gate = s.knowledge_gate.write().await;
    // Stop before committing: no concurrent generation or queued speech can retain deleted context.
    s.invalidate_knowledge().await;
    let now = crate::viewers::utc_ms() as i64;
    let result = match req.operation.as_str() {
        "correct" => {
            store(&s)?
                .admin_correct(
                    &s.config.viewers.scope_id,
                    &request,
                    req.value.as_deref().unwrap(),
                    now,
                )
                .await
        }
        "delete" => {
            store(&s)?
                .admin_delete(&s.config.viewers.scope_id, &request, now)
                .await
        }
        _ => {
            store(&s)?
                .admin_freeze(
                    &s.config.viewers.scope_id,
                    &request,
                    req.frozen.unwrap(),
                    now,
                )
                .await
        }
    };
    result.map_err(|_| {
        ApiError::new(
            StatusCode::CONFLICT,
            "memory_version_conflict",
            "操作未完成，请刷新记忆版本后重试",
        )
    })?;
    Ok(Json(serde_json::json!({"updated":true})))
}
pub async fn status(State(s): State<AppState>) -> Result<Json<dto::MemoryJobsStatus>, ApiError> {
    let r = store(&s)?
        .status(&s.config.viewers.scope_id)
        .await
        .map_err(|_| unavailable())?;
    Ok(Json(dto::MemoryJobsStatus {
        pending: r.pending,
        running: r.running,
        failed: r.failed,
        embedding_pending: r.embedding_pending,
        embedding_failed: r.embedding_failed,
    }))
}

pub async fn retry(
    State(s): State<AppState>,
    Json(r): Json<meowlive_protocol::relationships::KnowledgeMaintenanceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    maintenance(s, r, false).await
}
pub async fn rebuild_vectors(
    State(s): State<AppState>,
    Json(r): Json<meowlive_protocol::relationships::KnowledgeMaintenanceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    maintenance(s, r, true).await
}
async fn maintenance(
    s: AppState,
    r: meowlive_protocol::relationships::KnowledgeMaintenanceRequest,
    rebuild: bool,
) -> Result<Json<serde_json::Value>, ApiError> {
    id(&r.request_key)?;
    if r.reason.trim().is_empty() || r.reason.len() > 1000 {
        return Err(invalid());
    }
    let request = meowlive_application::ports::memory_store::MemoryMaintenanceRequest {
        request_key: r.request_key,
        reason: r.reason,
        actor: "single_admin".into(),
    };
    let store = store(&s)?;
    let now = crate::viewers::utc_ms() as i64;
    let result = if rebuild {
        store
            .rebuild_vectors(&s.config.viewers.scope_id, &request, now)
            .await
    } else {
        store
            .retry_failed(&s.config.viewers.scope_id, &request, now)
            .await
    };
    result.map_err(|_| {
        ApiError::new(
            StatusCode::CONFLICT,
            "memory_operation_conflict",
            "恢复操作冲突，请使用相同参数重试或重新提交",
        )
    })?;
    Ok(Json(serde_json::json!({"updated":true})))
}
