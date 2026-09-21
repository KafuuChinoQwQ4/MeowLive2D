//! Agent 运行设置、用量与阶段查询；文件访问在线程池中进行且不返回搜索密钥。
use crate::{llm_runtime::UsageQuery, state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{
        Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::StatusCode,
};
use meowlive_protocol::llm_runtime::{
    AgentActivitySnapshot, AgentRuntimeSettingsRequest, AgentRuntimeSettingsSnapshot,
    LlmUsageSnapshot,
};

pub async fn settings(State(state): State<AppState>) -> Json<AgentRuntimeSettingsSnapshot> {
    Json(state.llm_runtime.snapshot())
}
pub async fn save(
    State(state): State<AppState>,
    body: Result<Json<AgentRuntimeSettingsRequest>, JsonRejection>,
) -> Result<Json<AgentRuntimeSettingsSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_agent_runtime",
            "Agent 运行设置字段无效",
        )
    })?;
    let store = state.llm_runtime;
    tokio::task::spawn_blocking(move || {
        store.save(request).map_err(|message| {
            let storage = message.starts_with("无法保存");
            ApiError::new(
                if storage {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    StatusCode::BAD_REQUEST
                },
                if storage {
                    "agent_runtime_save_failed"
                } else {
                    "invalid_agent_runtime"
                },
                message,
            )
        })
    })
    .await
    .map_err(|_| unavailable())?
    .map(Json)
}
pub async fn usage(
    State(state): State<AppState>,
    query: Result<Query<UsageQuery>, QueryRejection>,
) -> Result<Json<LlmUsageSnapshot>, ApiError> {
    let Query(query) = query.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_usage_query",
            "用量查询参数无效",
        )
    })?;
    query.validate().map_err(|message| {
        ApiError::new(StatusCode::BAD_REQUEST, "invalid_usage_query", message)
    })?;
    tokio::task::spawn_blocking(move || state.llm_runtime.usage(&query))
        .await
        .map(Json)
        .map_err(|_| unavailable())
}
pub async fn activity(State(state): State<AppState>) -> Json<AgentActivitySnapshot> {
    Json(state.llm_runtime.activity())
}
fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "agent_runtime_unavailable",
        "Agent 运行存储暂不可用",
    )
}
