//! 直播连接 HTTP 输入边界；状态所有权和异步任务由主服务管理。
use super::error::ApiError;
use crate::state::AppState;
use axum::{Json, extract::State};
use meowlive_protocol::live::LiveConnectionSnapshot;

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
