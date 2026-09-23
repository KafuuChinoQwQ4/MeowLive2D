//! Agent 调度快照和 trace 历史的只读管理员接口。
use crate::{state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{StatusCode, header},
};
use meowlive_protocol::agent_observability::{AgentSchedulerSnapshot, AgentTrace, AgentTraceList};
use serde::Deserialize;

type NoStoreJson<T> = ([(header::HeaderName, &'static str); 1], Json<T>);

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceQuery {
    limit: Option<usize>,
    before_ms: Option<u64>,
}

pub async fn scheduler(State(state): State<AppState>) -> NoStoreJson<AgentSchedulerSnapshot> {
    no_store(crate::agent::admission::assess(&state).await)
}

pub async fn list(
    State(state): State<AppState>,
    query: Result<Query<TraceQuery>, QueryRejection>,
) -> Result<NoStoreJson<AgentTraceList>, ApiError> {
    let Query(query) = query.map_err(|_| invalid_query())?;
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(invalid_query());
    }
    let store = state.agent_observability;
    tokio::task::spawn_blocking(move || store.list(limit, query.before_ms))
        .await
        .map(no_store)
        .map_err(|_| unavailable())
}

pub async fn detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<NoStoreJson<AgentTrace>, ApiError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_agent_trace_id",
            "Agent trace 标识无效",
        ));
    }
    let store = state.agent_observability;
    let trace = tokio::task::spawn_blocking(move || store.get(&id))
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "agent_trace_not_found",
                "没有找到这条 Agent trace",
            )
        })?;
    Ok(no_store(trace))
}

fn no_store<T>(value: T) -> NoStoreJson<T> {
    ([(header::CACHE_CONTROL, "no-store")], Json(value))
}

fn invalid_query() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "invalid_agent_trace_query",
        "Agent trace 查询参数无效",
    )
}

fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "agent_trace_unavailable",
        "Agent trace 历史暂不可用",
    )
}
