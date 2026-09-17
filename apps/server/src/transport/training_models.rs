//! 模型驻留状态独立于 TTS 服务和已保存的训练版本。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_adapters::speech::model_synthesizer::request_model_runtime;
use meowlive_protocol::training_runtime::{
    TrainingModelRuntimeRequest, TrainingModelRuntimeSnapshot,
};

pub async fn status(State(state): State<AppState>) -> Json<TrainingModelRuntimeSnapshot> {
    let result = match &state.model_synthesizer {
        Some(synth) => synth.model_status().await,
        None => request_model_runtime(&state.config.speech.base_url, None).await,
    };
    Json(match result {
        Ok(value) => TrainingModelRuntimeSnapshot {
            supported: value.supported,
            state: value.state,
            message: value.message,
        },
        Err(_) => TrainingModelRuntimeSnapshot {
            supported: true,
            state: "unavailable".into(),
            message: "TTS 服务未就绪，请先在启动与运行页面启动 TTS".into(),
        },
    })
}

pub async fn control(
    State(state): State<AppState>,
    body: Result<Json<TrainingModelRuntimeRequest>, JsonRejection>,
) -> Result<Json<TrainingModelRuntimeSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "需要 enabled 布尔值",
        )
    })?;
    let synth = state.model_synthesizer.clone();
    let lease = state.acquire_model_selection().await?;
    let base = state.config.speech.base_url.clone();
    let measurement = state.measurement.clone();
    // A disconnected browser cannot release the fence before loading/unloading ends.
    let result = tokio::spawn(async move {
        let _lease = lease;
        let result = match synth {
            Some(synth) => synth.set_models_enabled(request.enabled).await,
            None => {
                let status = request_model_runtime(&base, None).await?;
                if !status.supported {
                    return Err(meowlive_application::ports::speech::SynthesisError::new(
                        "当前 TTS 不支持模型开关，请启动项目受管 TTS",
                    ));
                }
                request_model_runtime(&base, Some(request.enabled)).await
            }
        }?;
        if !result.supported
            || result.state
                != if request.enabled {
                    "loaded"
                } else {
                    "unloaded"
                }
        {
            return Err(meowlive_application::ports::speech::SynthesisError::new(
                "引擎未确认模型启停，请刷新状态后重试",
            ));
        }
        *measurement.lock().await = None;
        Ok(result)
    })
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "models_failed",
            "模型启停未完成，请刷新状态",
        )
    })?
    .map_err(|e| ApiError::new(StatusCode::CONFLICT, "models_failed", e.to_string()))?;
    Ok(Json(TrainingModelRuntimeSnapshot {
        supported: result.supported,
        state: result.state,
        message: result.message,
    }))
}
