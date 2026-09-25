//! 控制面板的 LLM 配置与显式连接测试；不回传提供商响应或密钥。
use crate::{state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_application::ports::{
    llm::DecisionRequest, llm_runtime::ModelOptions, reasoning::ReasoningEffort,
};
use meowlive_protocol::llm::{
    LlmModelOption, LlmModelsRequest, LlmModelsResult, LlmProfileCreateRequest,
    LlmProfileIdRequest, LlmProfileRenameRequest, LlmReasoningRequest, LlmReasoningResult,
    LlmSettingsRequest, LlmSettingsSnapshot, LlmTestResult,
};

pub async fn reasoning(
    body: Result<Json<LlmReasoningRequest>, JsonRejection>,
) -> Result<Json<LlmReasoningResult>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("推理强度预览字段无效".into()))?;
    if request.provider.trim().is_empty()
        || request.provider.len() > 64
        || request.provider.chars().any(char::is_control)
        || request.model.trim().is_empty()
        || request.model.len() > 128
        || request.model.chars().any(char::is_control)
        || !(64..=65536).contains(&request.max_tokens)
    {
        return Err(invalid(
            "推理强度预览的服务商、模型或 token 上限无效".into(),
        ));
    }
    let format = request
        .api_format
        .parse()
        .map_err(|_| invalid("LLM API 格式无效".into()))?;
    let requested = request
        .reasoning_effort
        .parse::<ReasoningEffort>()
        .map_err(|_| {
            invalid("推理强度须为 default、minimal、low、medium、high、xhigh、max 或 ultra".into())
        })?;
    let resolution = meowlive_adapters::llm::reasoning::resolve_reasoning(
        &request.provider,
        format,
        &request.model,
        requested,
        request.max_tokens,
    );
    Ok(Json(LlmReasoningResult {
        requested: resolution.requested.as_str().into(),
        effective: resolution.effective.map(|level| level.as_str().into()),
        supported: resolution
            .supported
            .iter()
            .map(|level| level.as_str().into())
            .collect(),
        strategy: resolution.strategy,
        budget_tokens: resolution.budget_tokens,
        note: resolution.note,
        error: resolution.error,
    }))
}

pub async fn models(
    State(state): State<AppState>,
    body: Result<Json<LlmModelsRequest>, JsonRejection>,
) -> Result<Json<LlmModelsResult>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("获取模型所需的连接字段无效".into()))?;
    let _permit = state.llm_settings.tests.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::CONFLICT,
            "llm_test_busy",
            "已有模型列表请求或连接测试正在进行",
        )
    })?;
    let config = state
        .llm_settings
        .models_candidate(request)
        .await
        .map_err(invalid)?;
    let catalog = meowlive_adapters::llm::models::list_models(config)
        .await
        .map_err(|error| {
            ApiError::new(StatusCode::BAD_GATEWAY, "llm_models_failed", error.message)
        })?;
    Ok(Json(LlmModelsResult {
        base_url: catalog.base_url,
        models: catalog
            .models
            .into_iter()
            .map(|model| LlmModelOption {
                id: model.id,
                name: model.name,
            })
            .collect(),
    }))
}

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
pub async fn create_profile(
    State(state): State<AppState>,
    body: Result<Json<LlmProfileCreateRequest>, JsonRejection>,
) -> Result<Json<LlmSettingsSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("LLM 配置字段无效".into()))?;
    let config = state
        .llm_settings
        .profile_candidate(request.clone())
        .await
        .map_err(invalid)?;
    let mut combined = state.config.as_ref().clone();
    combined.llm = config;
    combined.validate().map_err(invalid)?;
    state
        .llm_settings
        .create_profile(request)
        .await
        .map(Json)
        .map_err(invalid)
}
pub async fn select_profile(
    State(state): State<AppState>,
    body: Result<Json<LlmProfileIdRequest>, JsonRejection>,
) -> Result<Json<LlmSettingsSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("LLM 配置标识无效".into()))?;
    state
        .llm_settings
        .select_profile(request)
        .await
        .map(Json)
        .map_err(invalid)
}
pub async fn rename_profile(
    State(state): State<AppState>,
    body: Result<Json<LlmProfileRenameRequest>, JsonRejection>,
) -> Result<Json<LlmSettingsSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("LLM 配置标题无效".into()))?;
    state
        .llm_settings
        .rename_profile(request)
        .await
        .map(Json)
        .map_err(invalid)
}
pub async fn delete_profile(
    State(state): State<AppState>,
    body: Result<Json<LlmProfileIdRequest>, JsonRejection>,
) -> Result<Json<LlmSettingsSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("LLM 配置标识无效".into()))?;
    state
        .llm_settings
        .delete_profile(request)
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
        .turn(
            DecisionRequest {
                memory_context: vec![],
                persona: "你是一位中文主播，请输出一条简短问候以测试连接。".into(),
                system_prompt: String::new(),
                topic: "连接测试".into(),
                events: vec![],
                history: vec![],
            },
            ModelOptions {
                reasoning_effort: config
                    .reasoning_effort
                    .parse()
                    .map_err(|_| invalid("推理强度配置无效".into()))?,
                reasoning_provider: config.provider.clone(),
                ..ModelOptions::default()
            },
        )
        .await
        .map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, "llm_test_failed", e.message))?;
    Ok(Json(LlmTestResult {
        message: "连接成功，模型已返回有效的 Agent 决策；测试不会保存配置或播放声音。".into(),
    }))
}
fn invalid(message: String) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "invalid_llm_settings", message)
}
