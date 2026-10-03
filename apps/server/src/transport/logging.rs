//! HTTP 状态只关联固定分类；不保存请求地址、正文或错误详情。
use crate::{
    logs::{LogCategory, LogEvent},
    state::AppState,
};
use axum::{
    extract::{Request, State},
    http::Method,
    middleware::Next,
    response::Response,
};
pub async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let excluded = request.uri().path() == "/api/logs";
    let category = LogCategory::from_path(request.uri().path());
    let mutation = matches!(
        *request.method(),
        Method::POST | Method::DELETE | Method::PUT | Method::PATCH
    );
    let response = next.run(request).await;
    let event = if response.status().is_server_error() {
        Some(LogEvent::HttpServerError)
    } else if response.status().is_client_error() {
        Some(LogEvent::HttpClientError)
    } else if mutation && response.status().is_success() {
        Some(LogEvent::OperationCompleted)
    } else {
        None
    };
    if !excluded {
        if let Some(event) = event {
            state.logs.record_in(event, Some(category));
        }
    }
    response
}
