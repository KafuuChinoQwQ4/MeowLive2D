//! 面板 HTTP 接口；用例与状态机留在 application。
use super::{error::ApiError, mapping, websocket};
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{Method, StatusCode, header},
    routing::{get, post},
};
use meowlive_application::speech::SpeechQueueError;
use meowlive_domain::{speech::SpeechText, voice::VoiceId};
use meowlive_protocol::control::{ServerStatus, SpeechRequest, SpeechSnapshot};
use tower_http::cors::CorsLayer;

pub fn router(state: AppState) -> Router {
    let origins: Vec<_> = state
        .config
        .server
        .allowed_origins
        .iter()
        .filter_map(|s| s.parse::<header::HeaderValue>().ok())
        .collect();
    Router::new()
        .route(
            "/api/admin/viewers/merge/preview",
            post(super::viewer_merge::preview),
        )
        .route(
            "/api/admin/viewers/merge/apply",
            post(super::viewer_merge::apply),
        )
        .route(
            "/api/admin/viewers/{id}/relationships",
            get(super::relationships::list),
        )
        .route(
            "/api/admin/relationships",
            post(super::relationships::create),
        )
        .route(
            "/api/admin/relationships/{id}",
            post(super::relationships::change),
        )
        .route("/api/admin/graph/status", get(super::relationships::status))
        .route(
            "/api/admin/graph/rebuild",
            post(super::relationships::rebuild),
        )
        .route("/api/admin/viewers", get(super::viewers::viewers))
        .route("/api/admin/events", get(super::viewers::events))
        .route("/api/admin/viewers/{id}/memories", get(super::memory::list))
        .route(
            "/api/admin/viewers/{id}/memories/{memory}",
            post(super::memory::mutate),
        )
        .route("/api/admin/memories/status", get(super::memory::status))
        .route("/api/admin/memories/retry", post(super::memory::retry))
        .route(
            "/api/admin/memories/rebuild-vectors",
            post(super::memory::rebuild_vectors),
        )
        .route("/api/admin/viewers/{id}", get(super::companionship::detail))
        .route(
            "/api/admin/viewers/{id}/adjust",
            post(super::companionship::adjust),
        )
        .route(
            "/api/admin/viewers/{id}/reverse",
            post(super::companionship::reverse),
        )
        .route(
            "/api/admin/viewers/{id}/gifts/confirm",
            post(super::companionship::confirm_gift),
        )
        .route(
            "/api/admin/companionship/status",
            get(super::companionship::health),
        )
        .route(
            "/api/admin/session",
            get(super::auth::status)
                .post(super::auth::login)
                .delete(super::auth::logout),
        )
        .route("/api/status", get(status))
        .route("/api/health", get(health))
        .route(
            "/api/llm/settings",
            get(super::llm::settings).post(super::llm::save),
        )
        .route(
            "/api/agent/runtime",
            get(super::llm_runtime::settings)
                .post(super::llm_runtime::save)
                .layer(DefaultBodyLimit::max(512 * 1024)),
        )
        .route("/api/llm/usage", get(super::llm_runtime::usage))
        .route("/api/agent/activity", get(super::llm_runtime::activity))
        .route(
            "/api/agent/scheduler",
            get(super::agent_observability::scheduler),
        )
        .route("/api/agent/traces", get(super::agent_observability::list))
        .route(
            "/api/agent/traces/{id}",
            get(super::agent_observability::detail),
        )
        .route("/api/llm/test", post(super::llm::test))
        .route("/api/llm/reasoning", post(super::llm::reasoning))
        .route("/api/llm/models", post(super::llm::models))
        .route(
            "/api/obs",
            get(super::obs::status).post(super::obs::control),
        )
        .route("/api/training", get(super::training::status))
        .route(
            "/api/training/models",
            get(super::training_models::status).post(super::training_models::control),
        )
        .route(
            "/api/training/jobs",
            post(super::training::create)
                .layer(DefaultBodyLimit::max(32 * 1024 * 1024 + 128 * 1024)),
        )
        .route(
            "/api/training/transcribe",
            post(super::training::transcribe)
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024 + 16 * 1024)),
        )
        .route("/api/training/cancel", post(super::training::cancel))
        .route("/api/training/activate", post(super::training::activate))
        .route("/api/training/save", post(super::training::save))
        .route("/api/training/delete", post(super::training::delete))
        .route("/api/training/audition", post(super::training::audition))
        .route("/api/runtime/preset", get(super::runtime::status))
        .route("/api/runtime/measure", post(super::runtime::measure))
        .route("/api/resources", get(super::resources::status))
        .route(
            "/api/voices",
            post(super::resources::upload)
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024 + 32 * 1024)),
        )
        .route("/api/voices/select", post(super::resources::select_voice))
        .route("/api/voices/delete", post(super::resources::delete_voice))
        .route(
            "/api/characters/delete",
            post(super::resources::delete_character),
        )
        .route(
            "/api/characters/save",
            post(super::resources::save_character),
        )
        .route(
            "/api/characters/select",
            post(super::resources::select_character),
        )
        .route("/api/characters/preview", post(super::resources::preview))
        .route("/api/desktop/resources", post(super::resources::desktop))
        .route(
            "/api/obs/settings",
            get(super::obs::settings).post(super::obs::save_settings),
        )
        .route("/api/live", get(super::live::status))
        .route(
            "/api/live/settings",
            get(super::live::settings).post(super::live::save_settings),
        )
        .route("/api/live/connect", post(super::live::connect))
        .route("/api/live/disconnect", post(super::live::disconnect))
        .route("/api/speech", post(speak))
        .route("/api/stop", post(stop))
        .route("/api/agent", get(super::agent::status))
        .route("/api/agent/settings", post(super::agent::settings))
        .route("/api/agent/pause", post(super::agent::pause))
        .route("/api/agent/resume", post(super::agent::resume))
        .route(
            "/api/events",
            post(super::agent::events).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route("/ws/control", get(websocket::control))
        .route("/ws/audio", get(websocket::audio))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            super::auth::guard,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            super::origin::guard,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
        )
        .with_state(state)
}

async fn status(State(state): State<AppState>) -> Json<ServerStatus> {
    Json(state.snapshot().await)
}
async fn stop(State(state): State<AppState>) -> Json<ServerStatus> {
    Json(state.stop().await)
}

async fn speak(
    State(state): State<AppState>,
    body: Result<Json<SpeechRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SpeechSnapshot>), ApiError> {
    let Json(request) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "需要 text 和 voice_id 字符串",
        )
    })?;
    SpeechText::new(&request.text)
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "invalid_text", e.to_string()))?;
    VoiceId::new(&request.voice_id)
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, "invalid_voice", e.to_string()))?;
    // Keep resolution and queue admission in the same resource edit fence.
    let _edit = state.resource_edits.clone().try_lock_owned().map_err(|_| {
        ApiError::new(
            StatusCode::CONFLICT,
            "resource_busy",
            "资源正在更新，请稍后播报",
        )
    })?;
    let resources = state.resources.clone();
    let voice_id = tokio::task::spawn_blocking(move || {
        let selected = if request.voice_id == "active" {
            resources.snapshot().active_voice_id
        } else {
            request.voice_id
        };
        resources.resolve_voice(&selected).map(|_| selected)
    })
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "resource_failed",
            "资源查询失败",
        )
    })?
    .map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "unknown_voice",
            "尚未选择可用音色，请先导入参考声音并选择使用",
        )
    })?;
    let mut inner = state.inner.lock().await;
    state.require_gpu_idle()?;
    if state
        .resource_changing
        .load(std::sync::atomic::Ordering::Acquire)
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "resource_busy",
            "正在切换角色，请稍后播报",
        ));
    }
    let task = inner
        .queue
        .enqueue(uuid::Uuid::new_v4().to_string(), request.text, voice_id)
        .map_err(|error| {
            let (status, code) = match error {
                SpeechQueueError::Disconnected => (StatusCode::CONFLICT, "bridge_disconnected"),
                SpeechQueueError::Full => (StatusCode::TOO_MANY_REQUESTS, "queue_full"),
                _ => (StatusCode::BAD_REQUEST, "invalid_request"),
            };
            ApiError::new(status, code, error.to_string())
        })?;
    state.wake.notify_one();
    Ok((StatusCode::ACCEPTED, Json(mapping::speech(&task))))
}

async fn health(State(state): State<AppState>) -> Json<meowlive_protocol::control::ServerHealth> {
    let inner = state.inner.lock().await;
    Json(meowlive_protocol::control::ServerHealth {
        service: "meowlive".into(),
        protocol_version: meowlive_protocol::PROTOCOL_VERSION,
        bridge_connected: inner.queue.is_connected(),
    })
}
