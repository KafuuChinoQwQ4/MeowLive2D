//! OBS 公开控制入口；通过独立执行端访问本机 OBS，不向面板暴露密码。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_protocol::{
    obs::{ObsOperation, ObsSettingsRequest, ObsSettingsSnapshot, ObsSnapshot},
    resources::{DesktopResourceOperation, DesktopResourceResult},
};

pub async fn settings(
    State(state): State<AppState>,
) -> Result<Json<ObsSettingsSnapshot>, ApiError> {
    settings_result(
        state
            .desktop_resource(DesktopResourceOperation::ObsSettings)
            .await?,
    )
    .map(Json)
}

pub async fn save_settings(
    State(state): State<AppState>,
    body: Result<Json<ObsSettingsRequest>, JsonRejection>,
) -> Result<Json<ObsSettingsSnapshot>, ApiError> {
    let Json(settings) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_obs_settings",
            "OBS 设置格式无效",
        )
    })?;
    if !valid_endpoint(&settings.websocket_url)
        || settings.password.as_deref().is_some_and(|value| {
            value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control)
        })
        || (settings.clear_password && settings.password.is_some())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_obs_settings",
            "请使用本机 IP 的 OBS 地址，并选择输入或清除密码",
        ));
    }
    settings_result(
        state
            .desktop_resource(DesktopResourceOperation::SaveObsSettings { settings })
            .await?,
    )
    .map(Json)
}

fn settings_result(result: DesktopResourceResult) -> Result<ObsSettingsSnapshot, ApiError> {
    match result {
        DesktopResourceResult::ObsSettings { settings }
            if valid_endpoint(&settings.websocket_url) =>
        {
            Ok(settings)
        }
        DesktopResourceResult::Error { message, .. } => Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "obs_settings_failed",
            message.chars().take(1000).collect::<String>(),
        )),
        _ => Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "invalid_obs_response",
            "桌面返回的 OBS 设置无效",
        )),
    }
}

fn valid_endpoint(value: &str) -> bool {
    if value.len() > 256 || value.chars().any(char::is_whitespace) {
        return false;
    }
    let Some(endpoint) = value.strip_prefix("ws://") else {
        return false;
    };
    let authority = endpoint.strip_suffix('/').unwrap_or(endpoint);
    if authority.contains(['@', '\\', '?', '#', '/']) {
        return false;
    }
    let (host, port) = if let Some(ipv6) = authority.strip_prefix('[') {
        let Some((host, suffix)) = ipv6.split_once(']') else {
            return false;
        };
        (host, suffix)
    } else {
        let split = authority.find(':').unwrap_or(authority.len());
        authority.split_at(split)
    };
    if !host
        .parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
    {
        return false;
    }
    port.is_empty()
        || port.strip_prefix(':').is_some_and(|port| {
            !port.is_empty()
                && port.bytes().all(|c| c.is_ascii_digit())
                && port.parse::<u16>().is_ok()
        })
}

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
