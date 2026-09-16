//! 对外稳定错误响应，避免将框架或引擎错误正文直接公开。
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use meowlive_protocol::control::ErrorResponse;

#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub &'static str, pub String);
impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self(status, code, message.into())
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(ErrorResponse {
                code: self.1.into(),
                message: self.2,
            }),
        )
            .into_response()
    }
}
