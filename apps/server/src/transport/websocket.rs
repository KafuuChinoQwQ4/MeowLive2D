//! 双 WebSocket 的连接准入；音频连接必须绑定当前控制连接。
use super::{bridge, error::ApiError};
use crate::state::{AppState, Bridge};
use axum::{
    extract::{Query, State, ws::WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub async fn control(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    super::origin::validate(&state, &headers)?;
    let mut inner = state.inner.lock().await;
    if inner.bridge.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "bridge_busy",
            "已有执行端连接",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let (sender, receiver) = mpsc::channel(16);
    let cancel = CancellationToken::new();
    inner.queue.stop();
    inner.generation_cancel.cancel();
    inner.generation_cancel = CancellationToken::new();
    inner.bridge = Some(Bridge {
        id: id.clone(),
        control: sender,
        audio: None,
        cancel: cancel.clone(),
    });
    drop(inner);
    let failed_state = state.clone();
    let failed_id = id.clone();
    Ok(ws
        .max_message_size(64 * 1024)
        .max_frame_size(64 * 1024)
        .on_failed_upgrade(move |_| {
            tokio::spawn(async move {
                failed_state.disconnect(&failed_id).await;
            });
        })
        .on_upgrade(move |socket| bridge::control(socket, state, id, receiver, cancel)))
}

#[derive(Deserialize)]
pub struct AudioPair {
    session_id: String,
    bridge_id: String,
}

pub async fn audio(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(pair): Query<AudioPair>,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    super::origin::validate(&state, &headers)?;
    let mut inner = state.inner.lock().await;
    let current = inner
        .bridge
        .as_mut()
        .filter(|b| b.id == pair.bridge_id && pair.session_id == *state.session_id)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "invalid_pair",
                "音频连接与当前执行端不匹配",
            )
        })?;
    if current.audio.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "audio_busy",
            "音频连接已配对",
        ));
    }
    let (sender, receiver) = mpsc::channel(8);
    current.audio = Some(sender);
    let cancel = current.cancel.clone();
    drop(inner);
    let failed_state = state.clone();
    let failed_id = pair.bridge_id.clone();
    Ok(ws
        .max_message_size(1024)
        .max_frame_size(1024)
        .on_failed_upgrade(move |_| {
            tokio::spawn(async move {
                failed_state.disconnect(&failed_id).await;
            });
        })
        .on_upgrade(move |socket| bridge::audio(socket, state, pair.bridge_id, receiver, cancel)))
}
