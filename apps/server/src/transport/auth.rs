//! 管理员会话端点与 HTTP、WebSocket 角色认证中间件。
use super::error::ApiError;
use crate::{auth::LoginError, state::AppState};
use axum::{
    Json,
    extract::{Request, State, rejection::JsonRejection},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use meowlive_protocol::auth::{AdminSessionRequest, AdminSessionStatus, AdminSessionToken};

type NoStoreJson<T> = ([(header::HeaderName, &'static str); 1], Json<T>);

pub async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> NoStoreJson<AdminSessionStatus> {
    let authenticated = !state.auth.enabled()
        || bearer(&headers).is_some_and(|token| state.auth.is_admin_session(token));
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(AdminSessionStatus {
            enabled: state.auth.enabled(),
            authenticated,
        }),
    )
}

pub async fn login(
    State(state): State<AppState>,
    body: Result<Json<AdminSessionRequest>, JsonRejection>,
) -> Result<NoStoreJson<AdminSessionToken>, ApiError> {
    let Json(request) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "需要 token 字符串",
        )
    })?;
    let session = state.auth.login(&request.token).map_err(login_error)?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(AdminSessionToken {
            token: session.token,
            expires_in_seconds: session.expires_in_seconds,
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let token = bearer(&headers).ok_or_else(authentication_required)?;
    if !state.auth.revoke(token) {
        return Err(authentication_required());
    }
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}

pub async fn guard(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if request.method() == Method::OPTIONS {
        return Ok(next.run(request).await);
    }
    let path = request.uri().path();
    let method = request.method();
    if path == "/api/admin/session" && (method == Method::GET || method == Method::POST) {
        return Ok(next.run(request).await);
    }
    if path == "/api/health" && method == Method::GET {
        return Ok(next.run(request).await);
    }
    // The single-user control panel belongs to the software owner. Authentication
    // is an explicit deployment option, independent of persistent viewer storage.
    if !state.auth.enabled() {
        return Ok(next.run(request).await);
    }

    let required = if path == "/ws/control" || path == "/ws/audio" {
        Some(Role::Device)
    } else if path.starts_with("/api/") {
        Some(Role::Admin)
    } else {
        None
    };
    let Some(required) = required else {
        return Ok(next.run(request).await);
    };
    if !state.auth.configured() {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "auth_locked",
            "认证凭据尚未加载",
        ));
    }
    let token = bearer(request.headers()).ok_or_else(authentication_required)?;
    let authenticated = match required {
        Role::Admin => state.auth.is_admin_session(token),
        Role::Device => state.auth.is_device(token),
    };
    if !authenticated {
        return Err(authentication_required());
    }
    Ok(next.run(request).await)
}

enum Role {
    Admin,
    Device,
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    (!token.is_empty() && token.trim() == token).then_some(token)
}

fn login_error(error: LoginError) -> ApiError {
    match error {
        LoginError::Disabled => {
            ApiError::new(StatusCode::FORBIDDEN, "auth_disabled", "管理员认证未启用")
        }
        LoginError::Locked => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "auth_locked",
            "认证凭据尚未加载",
        ),
        LoginError::Invalid => authentication_required(),
        LoginError::RateLimited => ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "login_rate_limited",
            "登录失败次数过多，请稍后重试",
        ),
    }
}

fn authentication_required() -> ApiError {
    ApiError::new(
        StatusCode::UNAUTHORIZED,
        "authentication_required",
        "需要有效的 Bearer 凭据",
    )
}
