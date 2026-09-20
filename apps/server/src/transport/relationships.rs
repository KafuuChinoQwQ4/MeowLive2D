//! 管理员关系事实操作；服务端从 SQL 核对证据和版本。
use crate::{state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use meowlive_application::ports::relationships::*;
use meowlive_protocol::relationships as dto;
use serde::Deserialize;
fn invalid() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "invalid_relationship",
        "关系标识、类型或操作依据无效",
    )
}
fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "relationships_unavailable",
        "关系存储当前不可用",
    )
}
fn conflict() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "relationship_conflict",
        "操作未完成，请核对证据并刷新版本",
    )
}
fn store(s: &AppState) -> Result<&dyn RelationshipStore, ApiError> {
    s.relationship_store.as_deref().ok_or_else(unavailable)
}
fn validate(key: &str, reason: &str) -> Result<(), ApiError> {
    if uuid::Uuid::parse_str(key).is_err() || reason.trim().is_empty() || reason.len() > 1000 {
        Err(invalid())
    } else {
        Ok(())
    }
}
fn entity(e: dto::RelationshipEntity) -> Result<RelationEntity, ApiError> {
    Ok(RelationEntity {
        kind: match e.kind.as_str() {
            "viewer" => EntityKind::Viewer,
            "topic" => EntityKind::Topic,
            "activity" => EntityKind::Activity,
            "unresolved" => EntityKind::Unresolved,
            _ => return Err(invalid()),
        },
        id: e.id,
    })
}
fn entity_dto(e: RelationEntity) -> dto::RelationshipEntity {
    dto::RelationshipEntity {
        kind: match e.kind {
            EntityKind::Viewer => "viewer",
            EntityKind::Topic => "topic",
            EntityKind::Activity => "activity",
            EntityKind::Unresolved => "unresolved",
        }
        .into(),
        id: e.id,
    }
}
fn fact_dto(f: RelationshipFact) -> dto::ViewerRelationship {
    dto::ViewerRelationship {
        id: f.id,
        version: f.version,
        source: entity_dto(f.source),
        target: entity_dto(f.target),
        kind: match f.kind {
            RelationKind::Mention => "mention",
            RelationKind::Acquaintance => "acquaintance",
            RelationKind::Participated => "participated",
            RelationKind::SharedInterest => "shared_interest",
            RelationKind::Friend => "friend",
        }
        .into(),
        confirmation: match f.confirmation {
            RelationConfirmation::Claimed => "claimed",
            RelationConfirmation::Confirmed => "confirmed",
        }
        .into(),
        evidence: f
            .evidence
            .into_iter()
            .map(|e| dto::RelationshipEvidence {
                source: e.source,
                event_id: e.event_id,
                quote: e.quote,
            })
            .collect(),
        expires_at_ms: f.expires_at_ms,
        deleted: f.deleted,
    }
}
#[derive(Deserialize)]
pub struct Options {
    depth: Option<u8>,
}
pub async fn list(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<Options>,
) -> Result<Json<dto::RelationshipPage>, ApiError> {
    if uuid::Uuid::parse_str(&id).is_err() || !matches!(q.depth.unwrap_or(1), 1 | 2) {
        return Err(invalid());
    }
    let mut facts = store(&s)?
        .list_admin(&s.config.viewers.scope_id, &id, 100, 0)
        .await
        .map_err(|_| unavailable())?;
    let mut degraded = s.relationship_graph.read().await.is_none();
    if q.depth == Some(2) {
        if let Ok((related, fallback)) = s.relationships(&id, 2, 40).await {
            degraded = fallback;
            for fact in related {
                if !facts.iter().any(|f| f.id == fact.id) {
                    facts.push(fact);
                }
            }
        }
    }
    Ok(Json(dto::RelationshipPage {
        relationships: facts.into_iter().map(fact_dto).collect(),
        degraded,
    }))
}
pub async fn create(
    State(s): State<AppState>,
    Json(r): Json<dto::RelationshipCreateRequest>,
) -> Result<Json<dto::ViewerRelationship>, ApiError> {
    validate(&r.request_key, &r.reason)?;
    let input = RelationInput {
        source: entity(r.source)?,
        target: entity(r.target)?,
        kind: match r.kind.as_str() {
            "mention" => RelationKind::Mention,
            "acquaintance" => RelationKind::Acquaintance,
            "participated" => RelationKind::Participated,
            "shared_interest" => RelationKind::SharedInterest,
            "friend" => RelationKind::Friend,
            _ => return Err(invalid()),
        },
        evidence: r
            .evidence
            .into_iter()
            .map(|e| RelationEvidence {
                source: e.source,
                event_id: e.event_id,
                quote: e.quote,
            })
            .collect(),
        expires_at_ms: r.expires_at_ms,
        admin_confirmed: r.admin_confirmed,
        reason: r.reason,
        request_key: r.request_key,
    };
    let store = store(&s)?;
    let _gate = s.knowledge_gate.write().await;
    s.invalidate_knowledge().await;
    Ok(Json(fact_dto(
        store
            .create(&s.config.viewers.scope_id, &input, crate::viewers::utc_ms())
            .await
            .map_err(|_| conflict())?,
    )))
}
pub async fn change(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(r): Json<dto::RelationshipChangeRequest>,
) -> Result<Json<dto::ViewerRelationship>, ApiError> {
    validate(&r.request_key, &r.reason)?;
    if uuid::Uuid::parse_str(&id).is_err() || r.expected_version == 0 {
        return Err(invalid());
    }
    let action = match r.action.as_str() {
        "confirm" => RelationAction::Confirm,
        "revoke" => RelationAction::Revoke,
        "delete" => RelationAction::Delete,
        _ => return Err(invalid()),
    };
    let store = store(&s)?;
    let _gate = s.knowledge_gate.write().await;
    s.invalidate_knowledge().await;
    Ok(Json(fact_dto(
        store
            .change(
                &s.config.viewers.scope_id,
                &id,
                r.expected_version,
                action,
                &r.reason,
                &r.request_key,
                crate::viewers::utc_ms(),
            )
            .await
            .map_err(|_| conflict())?,
    )))
}
pub async fn status(State(s): State<AppState>) -> Result<Json<dto::GraphStatus>, ApiError> {
    let r = store(&s)?
        .status(&s.config.viewers.scope_id, crate::viewers::utc_ms())
        .await
        .map_err(|_| unavailable())?;
    Ok(Json(dto::GraphStatus {
        pending: r.pending,
        leased: r.leased,
        failed: r.failed,
        oldest_pending_age_ms: r.oldest_pending_age_ms,
        connected: s.relationship_graph.read().await.is_some(),
    }))
}
pub async fn rebuild(
    State(s): State<AppState>,
    Json(r): Json<dto::KnowledgeMaintenanceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validate(&r.request_key, &r.reason)?;
    let count = store(&s)?
        .rebuild(
            &s.config.viewers.scope_id,
            &r.request_key,
            &r.reason,
            crate::viewers::utc_ms(),
        )
        .await
        .map_err(|_| conflict())?;
    Ok(Json(serde_json::json!({"queued":count})))
}
