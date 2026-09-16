//! Agent HTTP 输入边界；领域校验与取消由应用状态组装执行。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_protocol::agent::{AgentSettings, AgentSnapshot, EventBatchRequest, EventBatchResult};

pub async fn status(State(state): State<AppState>) -> Json<AgentSnapshot> {
    Json(state.agent_snapshot().await)
}
pub async fn pause(State(state): State<AppState>) -> Json<AgentSnapshot> {
    Json(state.pause_agent().await)
}
pub async fn resume(State(state): State<AppState>) -> Result<Json<AgentSnapshot>, ApiError> {
    state.resume_agent().await.map(Json)
}
pub async fn settings(
    State(state): State<AppState>,
    body: Result<Json<AgentSettings>, JsonRejection>,
) -> Result<Json<AgentSnapshot>, ApiError> {
    let Json(settings) = body.map_err(|_| invalid())?;
    state.configure_agent(settings).await.map(Json)
}
pub async fn events(
    State(state): State<AppState>,
    body: Result<Json<EventBatchRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<EventBatchResult>), ApiError> {
    let Json(batch) = body.map_err(|_| invalid())?;
    state
        .submit_events(batch)
        .await
        .map(|result| (StatusCode::ACCEPTED, Json(result)))
}
fn invalid() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "invalid_agent_request",
        "请求格式、字段或大小无效",
    )
}
