use meowlive_application::ports::llm::LlmError;
use reqwest::{Url, header::HeaderValue};

use super::ApiFormat;

pub(super) fn normalize_base(
    input: &str,
    format: ApiFormat,
    default_root_version: bool,
) -> Result<(Url, bool), LlmError> {
    if input.len() > 4096 || input.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid_url());
    }
    let mut base = Url::parse(input).map_err(|_| invalid_url())?;
    let raw_authority = input
        .split_once("://")
        .map(|(_, rest)| rest.split(['/', '?', '#', '\\']).next().unwrap_or_default());
    if !matches!(base.scheme(), "http" | "https")
        || base.host_str().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || raw_authority.is_none()
        || raw_authority.is_some_and(|authority| authority.contains('@'))
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err(invalid_url());
    }
    let path = base.path().trim_end_matches('/');
    let root_only = path.is_empty();
    let normalized = if root_only && default_root_version {
        if format == ApiFormat::GeminiGenerateContent {
            "/v1beta"
        } else {
            "/v1"
        }
    } else if let Some(path) = path.strip_suffix("/chat/completions") {
        path
    } else if let Some(path) = path.strip_suffix("/responses") {
        path
    } else if let Some(path) = path.strip_suffix("/messages") {
        path
    } else if let Some(path) = path.strip_suffix("/models") {
        path
    } else if let Some((prefix, model)) = path.rsplit_once("/models/") {
        if model.ends_with(":generateContent") && !model.contains('/') {
            prefix
        } else {
            path
        }
    } else {
        path
    }
    .to_owned();
    base.set_path(&normalized);
    Ok((
        base,
        root_only && default_root_version && format != ApiFormat::GeminiGenerateContent,
    ))
}

pub(super) fn authentication_header(
    api_key: Option<String>,
    format: ApiFormat,
) -> Result<Option<HeaderValue>, LlmError> {
    api_key
        .map(|key| {
            let invalid_key = || LlmError::new("模型服务 API Key 无效", false);
            if key.trim().is_empty() || key.len() > 4096 || key.chars().any(char::is_control) {
                return Err(invalid_key());
            }
            let value = if matches!(format, ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses) {
                format!("Bearer {key}")
            } else {
                key
            };
            let mut header = HeaderValue::from_str(&value).map_err(|_| invalid_key())?;
            header.set_sensitive(true);
            Ok(header)
        })
        .transpose()
}

fn invalid_url() -> LlmError {
    LlmError::new(
        "模型服务地址必须是有效的 HTTP 或 HTTPS 地址，且不得包含凭据、查询参数或片段",
        false,
    )
}
