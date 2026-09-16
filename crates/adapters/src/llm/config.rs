use std::{fmt, time::Duration};

use meowlive_application::ports::llm::LlmError;
use reqwest::{Url, header::HeaderValue};

#[derive(Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub max_response_bytes: usize,
    pub max_tokens: u32,
    pub json_mode: bool,
}

impl fmt::Debug for LlmConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LlmConfig")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("timeout", &self.timeout)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("max_tokens", &self.max_tokens)
            .field("json_mode", &self.json_mode)
            .finish()
    }
}

pub(super) struct ValidatedConfig {
    pub endpoint: Url,
    pub model: String,
    pub api_key: Option<HeaderValue>,
    pub timeout: Duration,
    pub max_response_bytes: usize,
    pub max_tokens: u32,
    pub json_mode: bool,
}

impl ValidatedConfig {
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        if config.base_url.len() > 4096 {
            return Err(config_error("LLM base URL exceeds the size limit"));
        }
        if config.base_url.chars().any(char::is_whitespace) {
            return Err(config_error("LLM base URL is invalid"));
        }
        let mut endpoint = Url::parse(&config.base_url)
            .map_err(|_| config_error("LLM base URL must be an HTTP or HTTPS URL"))?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || endpoint.host_str().is_none()
            || endpoint.authority().contains('@')
            || has_userinfo_syntax(&config.base_url)
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(config_error(
                "LLM base URL must not contain credentials, a query, or a fragment",
            ));
        }
        endpoint.set_path(&format!(
            "{}/chat/completions",
            endpoint.path().trim_end_matches('/')
        ));

        let model = config.model.trim();
        if model.is_empty() || model.len() > 128 || model.chars().any(char::is_control) {
            return Err(config_error("LLM model is invalid"));
        }
        if !(Duration::from_secs(1)..=Duration::from_secs(120)).contains(&config.timeout) {
            return Err(config_error(
                "LLM timeout must be between 1 and 120 seconds",
            ));
        }
        if !(1024..=1024 * 1024).contains(&config.max_response_bytes) {
            return Err(config_error(
                "LLM response size limit must be between 1024 bytes and 1 MiB",
            ));
        }
        if !(64..=4096).contains(&config.max_tokens) {
            return Err(config_error("LLM max tokens must be between 64 and 4096"));
        }
        let api_key = config
            .api_key
            .map(|key| {
                if key.trim().is_empty() || key.chars().any(char::is_control) {
                    return Err(config_error("LLM API key is invalid"));
                }
                HeaderValue::from_str(&format!("Bearer {key}"))
                    .map_err(|_| config_error("LLM API key is invalid"))
            })
            .transpose()?;

        Ok(Self {
            endpoint,
            model: model.to_owned(),
            api_key,
            timeout: config.timeout,
            max_response_bytes: config.max_response_bytes,
            max_tokens: config.max_tokens,
            json_mode: config.json_mode,
        })
    }
}

fn config_error(message: &'static str) -> LlmError {
    LlmError::new(message, false)
}

fn has_userinfo_syntax(input: &str) -> bool {
    let Some((_, after_scheme)) = input.split_once("://") else {
        return false;
    };
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    after_scheme[..authority_end].contains('@')
}
