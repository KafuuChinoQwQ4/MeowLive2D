//! 直播连接 HTTP 输入边界；状态所有权和异步任务由主服务管理。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_protocol::live::{LiveConnectionSnapshot, LiveSettingsRequest, LiveSettingsSnapshot};

pub async fn settings(State(state): State<AppState>) -> Json<LiveSettingsSnapshot> {
    Json(state.live_settings_snapshot().await)
}

pub async fn save_settings(
    State(state): State<AppState>,
    body: Result<Json<LiveSettingsRequest>, JsonRejection>,
) -> Result<Json<LiveSettingsSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_live_settings",
            "直播配置字段无效",
        )
    })?;
    state.save_live_settings(request).await.map(Json)
}

pub async fn status(State(state): State<AppState>) -> Json<LiveConnectionSnapshot> {
    Json(state.live_snapshot().await)
}
pub async fn connect(
    State(state): State<AppState>,
) -> Result<Json<LiveConnectionSnapshot>, ApiError> {
    state.connect_live().await.map(Json)
}
pub async fn disconnect(State(state): State<AppState>) -> Json<LiveConnectionSnapshot> {
    Json(state.disconnect_live().await)
}
