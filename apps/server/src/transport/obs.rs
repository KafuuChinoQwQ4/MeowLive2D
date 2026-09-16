//! OBS 公开控制入口；通过独立执行端访问本机 OBS，不向面板暴露密码。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_protocol::{
    obs::{ObsOperation, ObsSnapshot},
    resources::{DesktopResourceOperation, DesktopResourceResult},
};

pub async fn status(State(state): State<AppState>) -> Result<Json<ObsSnapshot>, ApiError> {
    execute(&state, ObsOperation::Status).await.map(Json)
}

pub async fn control(
    State(state): State<AppState>,
    body: Result<Json<ObsOperation>, JsonRejection>,
) -> Result<Json<ObsSnapshot>, ApiError> {
    let Json(operation) = body
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid_obs", "OBS 操作格式无效"))?;
    if let ObsOperation::SetScene { scene_name } = &operation {
        if !scene_name_valid(scene_name) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_obs",
                "OBS 场景名称无效",
            ));
        }
    }
    execute(&state, operation).await.map(Json)
}

fn scene_name_valid(value: &str) -> bool {
    !value.trim().is_empty() && value.chars().count() <= 256 && !value.chars().any(char::is_control)
}

async fn execute(state: &AppState, operation: ObsOperation) -> Result<ObsSnapshot, ApiError> {
    let result = state
        .desktop_resource(DesktopResourceOperation::Obs { operation })
        .await?;
    match result {
        DesktopResourceResult::Obs { snapshot } if valid_snapshot(&snapshot) => Ok(snapshot),
        DesktopResourceResult::Error { message, .. } => Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "obs_failed",
            message.chars().take(1000).collect::<String>(),
        )),
        _ => Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "invalid_obs_response",
            "桌面返回的 OBS 状态无效",
        )),
    }
}

fn valid_snapshot(snapshot: &ObsSnapshot) -> bool {
    if !snapshot.connected {
        return !snapshot.recording
            && snapshot.current_scene.is_empty()
            && snapshot.scenes.is_empty();
    }
    scene_name_valid(&snapshot.current_scene)
        && snapshot.scenes.len() <= 256
        && snapshot.scenes.iter().all(|name| scene_name_valid(name))
        && snapshot.scenes.contains(&snapshot.current_scene)
}
