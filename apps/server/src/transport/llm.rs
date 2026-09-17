//! 控制面板的 LLM 配置与显式连接测试；不回传提供商响应或密钥。
use crate::{state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_application::ports::llm::DecisionRequest;
use meowlive_protocol::llm::{LlmSettingsRequest, LlmSettingsSnapshot, LlmTestResult};

pub async fn settings(
    State(state): State<AppState>,
) -> Result<Json<LlmSettingsSnapshot>, ApiError> {
    state
        .llm_settings
        .snapshot()
        .await
        .map(Json)
        .map_err(invalid)
}
pub async fn save(
    State(state): State<AppState>,
    body: Result<Json<LlmSettingsRequest>, JsonRejection>,
) -> Result<Json<LlmSettingsSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("LLM 配置字段无效".into()))?;
    let config = state
        .llm_settings
        .candidate(request.clone())
        .await
        .map_err(invalid)?;
    let mut combined = state.config.as_ref().clone();
    combined.llm = config;
    combined.validate().map_err(invalid)?;
    state
        .llm_settings
        .save(request)
        .await
        .map(Json)
        .map_err(invalid)
}
pub async fn test(
    State(state): State<AppState>,
    body: Result<Json<LlmSettingsRequest>, JsonRejection>,
) -> Result<Json<LlmTestResult>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("LLM 配置字段无效".into()))?;
    let _permit = state.llm_settings.tests.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::CONFLICT,
            "llm_test_busy",
            "已有 LLM 连接测试正在进行",
        )
    })?;
    let config = state
        .llm_settings
        .candidate(request)
        .await
        .map_err(invalid)?;
    let model = crate::bootstrap::build_model(&config)
        .map_err(invalid)?
        .ok_or_else(|| invalid("LLM 未配置".into()))?;
    model
        .decide(DecisionRequest {
            persona: "你是一位中文主播，请输出一条简短问候以测试连接。".into(),
            topic: "连接测试".into(),
            events: vec![],
            history: vec![],
        })
        .await
        .map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, "llm_test_failed", e.message))?;
    Ok(Json(LlmTestResult {
        message: "连接成功，模型已返回有效的 Agent 决策；测试不会保存配置或播放声音。".into(),
    }))
}
fn invalid(message: String) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "invalid_llm_settings", message)
}
