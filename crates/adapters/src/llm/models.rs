//! 从供应商原生模型目录发现可选模型。

mod config;
mod response;

use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

use meowlive_application::ports::llm::LlmError;
use reqwest::{
    Client, Url,
    header::{self, HeaderValue},
};
use serde_json::Value;

use super::multi_provider::ApiFormat;
use config::{authentication_header, normalize_base};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 1000;
const MAX_PAGES: usize = 10;

pub struct ModelCatalogConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub api_format: ApiFormat,
    pub timeout: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AvailableModel {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCatalog {
    pub base_url: String,
    pub models: Vec<AvailableModel>,
}

/// 用于比较供应商目标的规范地址。根路径与显式版本前缀保持不同的密钥作用域。
pub fn canonical_base_url(base_url: &str, format: ApiFormat) -> Result<String, LlmError> {
    let (base, _) = normalize_base(base_url, format, false)?;
    Ok(base.as_str().trim_end_matches('/').to_owned())
}

pub async fn list_models(config: ModelCatalogConfig) -> Result<ModelCatalog, LlmError> {
    let (base, fallback) = normalize_base(&config.base_url, config.api_format, true)?;
    let api_key = authentication_header(config.api_key, config.api_format)?;
    if config.timeout.is_zero() || config.timeout > Duration::from_secs(30) {
        return Err(LlmError::new(
            "获取模型目录的超时时间必须大于零且不超过 30 秒",
            false,
        ));
    }
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(config.timeout)
        .build()
        .map_err(|_| LlmError::new("无法初始化模型目录连接", false))?;
    tokio::time::timeout(
        config.timeout,
        fetch_models(client, base, fallback, api_key, config.api_format),
    )
    .await
    .map_err(|_| LlmError::new("获取模型目录超时", true))?
}

async fn fetch_models(
    client: Client,
    mut base: Url,
    mut fallback: bool,
    api_key: Option<HeaderValue>,
    format: ApiFormat,
) -> Result<ModelCatalog, LlmError> {
    let mut models = BTreeMap::new();
    let mut cursor: Option<String> = None;
    let mut visited = HashSet::new();
    let mut total_bytes = 0;
    let mut total_entries = 0;
    let mut pages = 0;
    loop {
        let mut endpoint = base.clone();
        let path = format!("{}/models", endpoint.path().trim_end_matches('/'));
        endpoint.set_path(&path);
        if let Some(cursor) = &cursor {
            let key = if format == ApiFormat::GeminiGenerateContent {
                "pageToken"
            } else {
                "after_id"
            };
            endpoint.query_pairs_mut().append_pair(key, cursor);
        }
        let mut request = client
            .get(endpoint)
            .header(header::ACCEPT, "application/json");
        if format == ApiFormat::AnthropicMessages {
            request = request.header("anthropic-version", "2023-06-01");
        }
        if let Some(key) = &api_key {
            request = match format {
                ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses => {
                    request.header(header::AUTHORIZATION, key)
                }
                ApiFormat::AnthropicMessages => request.header("x-api-key", key),
                ApiFormat::GeminiGenerateContent => request.header("x-goog-api-key", key),
            };
        }
        let mut response = request.send().await.map_err(transport_error)?;
        if fallback && matches!(response.status().as_u16(), 404 | 405) {
            base.set_path("");
            fallback = false;
            continue;
        }
        fallback = false;
        if !response.status().is_success() {
            let status = response.status();
            return Err(LlmError::new(
                format!("模型目录服务返回 HTTP {}", status.as_u16()),
                status.as_u16() == 429 || status.is_server_error(),
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > (MAX_RESPONSE_BYTES - total_bytes) as u64)
        {
            return Err(response_limit());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if chunk.len() > MAX_RESPONSE_BYTES - total_bytes {
                return Err(response_limit());
            }
            total_bytes += chunk.len();
            bytes.extend_from_slice(&chunk);
        }
        let body: Value = serde_json::from_slice(&bytes).map_err(|_| invalid_response())?;
        let entries = response::entries(&body, format)?;
        if entries.len() > MAX_ENTRIES - total_entries {
            return Err(LlmError::new(
                "供应商模型目录超过 1000 个条目，无法获取完整列表",
                false,
            ));
        }
        total_entries += entries.len();
        for entry in entries {
            if let Some(model) = response::parse_model(entry, format)? {
                models.entry(model.id.clone()).or_insert(model);
            }
        }
        pages += 1;
        let Some(next_cursor) = response::next_cursor(&body, format)? else {
            break;
        };
        if !visited.insert(next_cursor.clone()) {
            return Err(LlmError::new(
                "供应商模型目录分页重复，无法获取完整列表",
                false,
            ));
        }
        if pages >= MAX_PAGES {
            return Err(LlmError::new(
                "供应商模型目录超过 10 页，无法获取完整列表",
                false,
            ));
        }
        cursor = Some(next_cursor);
    }
    Ok(ModelCatalog {
        base_url: base.as_str().trim_end_matches('/').to_owned(),
        models: models.into_values().collect(),
    })
}

fn response_limit() -> LlmError {
    LlmError::new("供应商模型目录超过 1 MiB，无法获取完整列表", false)
}

fn invalid_response() -> LlmError {
    LlmError::new("供应商返回的模型目录格式无效或不受支持", false)
}

fn transport_error(error: reqwest::Error) -> LlmError {
    if error.is_timeout() {
        LlmError::new("获取模型目录超时", true)
    } else {
        LlmError::new("无法连接模型目录服务", true)
    }
}
