//! 三端共用的语音合成设置接口。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_protocol::control::SpeechSettings;

pub async fn settings(State(state): State<AppState>) -> Json<SpeechSettings> {
    Json(state.speech_settings.snapshot())
}
pub async fn save(
    State(state): State<AppState>,
    body: Result<Json<SpeechSettings>, JsonRejection>,
) -> Result<Json<SpeechSettings>, ApiError> {
    let Json(settings) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "并行合成分句数须为 1～16 的整数",
        )
    })?;
    crate::speech_settings::validate(settings.sentence_batch_size)
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "invalid_request", e))?;
    tokio::task::spawn_blocking(move || state.speech_settings.save(settings))
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "save_failed",
                "语音合成设置保存任务失败",
            )
        })?
        .map(Json)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "save_failed", e))
}
