//! 浏览器请求来源校验；CORS 响应头本身不能阻止简单 POST 的副作用。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::ORIGIN},
    middleware::Next,
    response::Response,
};

pub fn validate(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    if let Some(origin) = headers.get(ORIGIN) {
        if !state
            .config
            .server
            .allowed_origins
            .iter()
            .any(|allowed| origin.as_bytes() == allowed.as_bytes())
        {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "invalid_origin",
                "不允许此请求来源",
            ));
        }
    }
    Ok(())
}

pub async fn guard(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    validate(&state, request.headers())?;
    Ok(next.run(request).await)
}
