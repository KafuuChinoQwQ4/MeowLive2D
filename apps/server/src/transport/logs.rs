use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
    http::{StatusCode, header},
};
use meowlive_protocol::log::{RuntimeLogListResponse, RuntimeLogRequest};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogQuery {
    pub level: Option<String>,
    pub source: Option<String>,
    pub category: Option<String>,
    pub query: Option<String>,
    pub limit: Option<usize>,
}

pub async fn list(
    State(state): State<AppState>,
    query: Result<Query<LogQuery>, QueryRejection>,
) -> Result<
    (
        [(header::HeaderName, &'static str); 1],
        Json<RuntimeLogListResponse>,
    ),
    ApiError,
> {
    let Query(query) = query
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid_query", "日志筛选参数无效"))?;
    let limit = query.limit.unwrap_or(100);
    if !(1..=1000).contains(&limit)
        || query.query.as_ref().is_some_and(|v| v.len() > 512)
        || query.source.as_ref().is_some_and(|v| v.len() > 64)
        || query.category.as_ref().is_some_and(|v| v.len() > 64)
        || query
            .level
            .as_deref()
            .is_some_and(|v| !matches!(v, "debug" | "info" | "warn" | "error"))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_query",
            "日志筛选参数无效",
        ));
    }
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(state.logs.list(
            query.level.as_deref(),
            query.source.as_deref(),
            query.category.as_deref(),
            query.query.as_deref(),
            limit,
        )),
    ))
}

pub async fn record(
    State(state): State<AppState>,
    body: Result<Json<RuntimeLogRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<
    (
        StatusCode,
        [(header::HeaderName, &'static str); 1],
        Json<meowlive_protocol::log::RuntimeLogEntry>,
    ),
    ApiError,
> {
    let Json(request) = body
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid_request", "日志事件无效"))?;
    let entry = state
        .logs
        .record_code(&request.code)
        .map_err(|code| ApiError::new(StatusCode::BAD_REQUEST, code, "日志事件代码不受支持"))?;
    Ok((
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, "no-store")],
        Json(entry),
    ))
}
