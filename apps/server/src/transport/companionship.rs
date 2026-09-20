//! 认证管理端的观众关系详情与可审计操作。
use crate::{state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use meowlive_application::ports::companionship as port;
use meowlive_protocol::{agent::GiftMetadataInput, companionship as dto};

fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "companionship_unavailable",
        "观众陪伴存储当前不可用",
    )
}
fn invalid() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "invalid_admin_operation",
        "观众标识、请求标识或调整依据无效",
    )
}
fn validate_id(id: &str) -> Result<(), ApiError> {
    uuid::Uuid::parse_str(id).map(|_| ()).map_err(|_| invalid())
}
fn validate_request(id: &str, key: &str, reason: &str) -> Result<(), ApiError> {
    validate_id(id)?;
    validate_id(key)?;
    if reason.trim().is_empty()
        || reason.chars().count() > 1000
        || reason.chars().any(|c| c.is_control())
    {
        return Err(invalid());
    }
    Ok(())
}
fn store(state: &AppState) -> Result<&dyn port::CompanionshipStore, ApiError> {
    state.companionship_store.as_deref().ok_or_else(unavailable)
}

pub async fn detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<dto::CompanionshipDetail>, ApiError> {
    validate_id(&id)?;
    let value = store(&state)?
        .detail(&state.config.viewers.scope_id, &id, 100)
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "viewer_not_found", "未找到该观众"))?;
    Ok(Json(dto::CompanionshipDetail {
        viewer_id: value.viewer_id,
        familiarity_milli: value.familiarity_milli,
        affinity_milli: value.affinity_milli,
        observed_days: value.observed_days,
        observed_sessions: value.observed_sessions,
        last_seen_at_ms: value.last_seen_at_ms,
        medal_level: value.medal_level,
        guard_level: value.guard_level,
        gifts: value
            .gifts
            .into_iter()
            .map(|g| dto::GiftLedgerEntry {
                source: g.source,
                event_id: g.event_id,
                name: g.name,
                count: g.count,
                metadata: g.metadata.map(|m| GiftMetadataInput {
                    price: m.price,
                    paid: m.paid,
                    medal_level: m.medal_level,
                    guard_level: m.guard_level,
                }),
                value_cents: g.value_cents,
                value_kind: g.value_kind,
                occurred_at_ms: g.occurred_at_ms,
            })
            .collect(),
        ledger: value
            .ledger
            .into_iter()
            .map(|l| dto::AffinityLedgerEntry {
                ledger_id: l.ledger_id,
                reversible: l.reversible,
                kind: l.kind,
                computed_delta_milli: l.computed_delta_milli,
                applied_delta_milli: l.applied_delta_milli,
                reason: l.reason,
                actor: l.actor,
                created_at_ms: l.created_at_ms,
                reversed_ledger_id: l.reversed_ledger_id,
            })
            .collect(),
    }))
}
pub async fn adjust(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<dto::AffinityAdjustmentRequest>,
) -> Result<Json<dto::AdminMutationResult>, ApiError> {
    validate_request(&id, &req.request_key, &req.reason)?;
    if req.delta_milli == 0 || req.delta_milli.unsigned_abs() > 100_000 {
        return Err(invalid());
    }
    let record_id = store(&state)?
        .adjust(
            &state.config.viewers.scope_id,
            &id,
            &port::AffinityAdjustment {
                request_key: req.request_key,
                reason: req.reason,
                delta_milli: req.delta_milli,
            },
            crate::viewers::utc_ms(),
        )
        .await
        .map_err(|_| operation_failed())?;
    Ok(Json(dto::AdminMutationResult { record_id }))
}
pub async fn reverse(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<dto::AffinityReversalRequest>,
) -> Result<Json<dto::AdminMutationResult>, ApiError> {
    validate_request(&id, &req.request_key, &req.reason)?;
    validate_id(&req.ledger_id)?;
    let record_id = store(&state)?
        .reverse(
            &state.config.viewers.scope_id,
            &id,
            &port::AffinityReversal {
                request_key: req.request_key,
                reason: req.reason,
                ledger_id: req.ledger_id,
            },
            crate::viewers::utc_ms(),
        )
        .await
        .map_err(|_| operation_failed())?;
    Ok(Json(dto::AdminMutationResult { record_id }))
}
pub async fn confirm_gift(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<dto::GiftConfirmationRequest>,
) -> Result<Json<dto::AdminMutationResult>, ApiError> {
    validate_request(&id, &req.request_key, &req.reason)?;
    if req.source.is_empty()
        || req.source.len() > 64
        || req.event_id.is_empty()
        || req.event_id.len() > 256
        || req.value_cents > 1_000_000_000_000
        || req.value_kind != "confirmed_paid_value"
    {
        return Err(invalid());
    }
    let record_id = store(&state)?
        .confirm_gift(
            &state.config.viewers.scope_id,
            &id,
            &port::GiftConfirmation {
                request_key: req.request_key,
                reason: req.reason,
                source: req.source,
                event_id: req.event_id,
                value_cents: req.value_cents,
                value_kind: req.value_kind,
            },
            crate::viewers::utc_ms(),
        )
        .await
        .map_err(|_| operation_failed())?;
    Ok(Json(dto::AdminMutationResult { record_id }))
}
fn operation_failed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "admin_operation_failed",
        "操作未完成：请刷新资料，检查来源、请求标识和依据后重试",
    )
}
pub async fn health(State(state): State<AppState>) -> Json<dto::CompanionshipHealth> {
    Json(dto::CompanionshipHealth {
        durable_receipts: state.receipt_journal.is_some(),
        failed_receipts: state.receipt_problems.lock().await.clone(),
        pending_receipts: state
            .receipts_pending
            .load(std::sync::atomic::Ordering::Relaxed),
        failed_receipt_attempts: state
            .receipt_failures
            .load(std::sync::atomic::Ordering::Relaxed),
    })
}
