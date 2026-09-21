//! 多提供商决策与原生运行层适配器。

use std::str::FromStr;

use meowlive_application::ports::llm::{
    AgentDecision, DecisionFuture, DecisionRequest, LanguageModel, LlmError,
};
use reqwest::{Client, RequestBuilder, header};
use serde_json::{Value, json};

use super::{
    config::{EndpointKind, ValidatedConfig},
    openai_compatible::OpenAiCompatible,
    prompt::build_prompt,
    response::parse_decision_content,
};

pub use super::config::LlmConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiFormat {
    OpenaiChat,
    OpenaiResponses,
    AnthropicMessages,
    GeminiGenerateContent,
}

impl FromStr for ApiFormat {
    type Err = LlmError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "openai_chat" => Ok(Self::OpenaiChat),
            "openai_responses" => Ok(Self::OpenaiResponses),
            "anthropic_messages" => Ok(Self::AnthropicMessages),
            "gemini_generate_content" => Ok(Self::GeminiGenerateContent),
            _ => Err(LlmError::new("LLM API format is invalid", false)),
        }
    }
}

pub struct MultiProvider {
    inner: Provider,
    runtime: super::runtime::RuntimeProvider,
}

enum Provider {
    OpenaiChat(OpenAiCompatible),
    Native(NativeProvider),
}

struct NativeProvider {
    client: Client,
    config: ValidatedConfig,
    format: ApiFormat,
}

impl MultiProvider {
    pub fn new(config: LlmConfig, format: ApiFormat) -> Result<Self, LlmError> {
        let runtime = super::runtime::RuntimeProvider::new(config.clone(), format)?;
        if format == ApiFormat::OpenaiChat {
            return Ok(Self {
                inner: Provider::OpenaiChat(OpenAiCompatible::new(config)?),
                runtime,
            });
        }

        let kind = match format {
            ApiFormat::OpenaiChat => unreachable!(),
            ApiFormat::OpenaiResponses => EndpointKind::OpenaiResponses,
            ApiFormat::AnthropicMessages => EndpointKind::AnthropicMessages,
            ApiFormat::GeminiGenerateContent => EndpointKind::GeminiGenerateContent,
        };
        let config = ValidatedConfig::new_for(config, kind)?;
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.timeout)
            .connect_timeout(config.timeout)
            .build()
            .map_err(|_| LlmError::new("could not initialize the LLM HTTP client", false))?;
        Ok(Self {
            runtime,
            inner: Provider::Native(NativeProvider {
                client,
                config,
                format,
            }),
        })
    }
}

impl LanguageModel for MultiProvider {
    fn turn(
        &self,
        request: DecisionRequest,
        options: meowlive_application::ports::llm_runtime::ModelOptions,
    ) -> meowlive_application::ports::llm_runtime::ModelTurnFuture<'_> {
        Box::pin(self.runtime.turn(request, options))
    }

    fn decide(&self, request: DecisionRequest) -> DecisionFuture<'_> {
        match &self.inner {
            Provider::OpenaiChat(adapter) => adapter.decide(request),
            Provider::Native(adapter) => Box::pin(adapter.request_decision(request)),
        }
    }
}

impl NativeProvider {
    async fn request_decision(&self, request: DecisionRequest) -> Result<AgentDecision, LlmError> {
        let prompt = build_prompt(&request)?;
        let payload = self.payload(prompt.system, prompt.user);
        let mut builder = self
            .client
            .post(self.config.endpoint.clone())
            .header(header::ACCEPT, "application/json")
            .json(&payload);
        if self.format == ApiFormat::AnthropicMessages {
            builder = builder.header("anthropic-version", "2023-06-01");
        }
        builder = self.authenticate(builder);

        let mut response = builder.send().await.map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(LlmError::new(
                format!("LLM service returned HTTP {}", status.as_u16()),
                status.as_u16() == 429 || status.is_server_error(),
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.config.max_response_bytes as u64)
        {
            return Err(response_limit_error());
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|length| length > self.config.max_response_bytes)
            {
                return Err(response_limit_error());
            }
            bytes.extend_from_slice(&chunk);
        }
        let content = match self.format {
            ApiFormat::OpenaiResponses => openai_response_content(&bytes)?,
            ApiFormat::AnthropicMessages => anthropic_content(&bytes)?,
            ApiFormat::GeminiGenerateContent => gemini_content(&bytes)?,
            ApiFormat::OpenaiChat => unreachable!(),
        };
        parse_decision_content(&content, &request)
    }

    fn authenticate(&self, mut builder: RequestBuilder) -> RequestBuilder {
        let Some(api_key) = &self.config.api_key else {
            return builder;
        };
        builder = match self.format {
            ApiFormat::OpenaiResponses => {
                let value = format!("Bearer {}", api_key.to_str().unwrap_or_default());
                builder.header(header::AUTHORIZATION, value)
            }
            ApiFormat::AnthropicMessages => builder.header("x-api-key", api_key),
            ApiFormat::GeminiGenerateContent => builder.header("x-goog-api-key", api_key),
            ApiFormat::OpenaiChat => unreachable!(),
        };
        builder
    }

    fn payload(&self, system: String, user: String) -> Value {
        match self.format {
            ApiFormat::OpenaiResponses => {
                let mut payload = json!({
                    "model": self.config.model,
                    "store": false,
                    "max_output_tokens": self.config.max_tokens,
                    "input": [
                        {"role": "system", "content": [{"type": "input_text", "text": system}]},
                        {"role": "user", "content": [{"type": "input_text", "text": user}]}
                    ]
                });
                if self.config.json_mode {
                    payload["text"] = json!({"format": {"type": "json_object"}});
                }
                payload
            }
            ApiFormat::AnthropicMessages => json!({
                "model": self.config.model,
                "stream": false,
                "max_tokens": self.config.max_tokens,
                "system": system,
                "messages": [{"role": "user", "content": user}]
            }),
            ApiFormat::GeminiGenerateContent => {
                let mut generation = json!({"maxOutputTokens": self.config.max_tokens});
                if self.config.json_mode {
                    generation["responseMimeType"] = json!("application/json");
                }
                json!({
                    "systemInstruction": {"parts": [{"text": system}]},
                    "contents": [{"role": "user", "parts": [{"text": user}]}],
                    "generationConfig": generation
                })
            }
            ApiFormat::OpenaiChat => unreachable!(),
        }
    }
}

fn openai_response_content(bytes: &[u8]) -> Result<String, LlmError> {
    let response: Value = serde_json::from_slice(bytes).map_err(|_| invalid_response())?;
    if response.get("status").and_then(Value::as_str) != Some("completed") {
        return Err(invalid_response());
    }
    let output = response
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)?;
    let mut result = None;
    for item in output {
        match item.get("type").and_then(Value::as_str) {
            Some("reasoning") => {}
            Some("message")
                if result.is_none()
                    && item.get("status").and_then(Value::as_str) == Some("completed")
                    && item.get("role").and_then(Value::as_str) == Some("assistant") =>
            {
                let parts = item
                    .get("content")
                    .and_then(Value::as_array)
                    .ok_or_else(invalid_response)?;
                if parts.len() != 1
                    || parts[0].get("type").and_then(Value::as_str) != Some("output_text")
                {
                    return Err(invalid_response());
                }
                result = parts[0]
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            _ => return Err(invalid_response()),
        }
    }
    result.ok_or_else(invalid_response)
}

fn anthropic_content(bytes: &[u8]) -> Result<String, LlmError> {
    let response: Value = serde_json::from_slice(bytes).map_err(|_| invalid_response())?;
    if response.get("type").and_then(Value::as_str) != Some("message")
        || response.get("role").and_then(Value::as_str) != Some("assistant")
        || !matches!(
            response.get("stop_reason").and_then(Value::as_str),
            Some("end_turn" | "stop_sequence")
        )
    {
        return Err(invalid_response());
    }
    let parts = response
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)?;
    let mut result = None;
    for part in parts {
        match part.get("type").and_then(Value::as_str) {
            Some("thinking" | "redacted_thinking") => {}
            Some("text") if result.is_none() => {
                result = part.get("text").and_then(Value::as_str).map(str::to_owned);
                if result.is_none() {
                    return Err(invalid_response());
                }
            }
            _ => return Err(invalid_response()),
        }
    }
    result.ok_or_else(invalid_response)
}

fn gemini_content(bytes: &[u8]) -> Result<String, LlmError> {
    let response: Value = serde_json::from_slice(bytes).map_err(|_| invalid_response())?;
    let candidates = response
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)?;
    if candidates.len() != 1
        || candidates[0].get("finishReason").and_then(Value::as_str) != Some("STOP")
    {
        return Err(invalid_response());
    }
    let content = candidates[0].get("content").ok_or_else(invalid_response)?;
    if content.get("role").and_then(Value::as_str) != Some("model") {
        return Err(invalid_response());
    }
    let parts = content
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)?;
    one_text_part(parts, |part| {
        part.get("thought").and_then(Value::as_bool) == Some(true)
    })
}

fn one_text_part(parts: &[Value], ignored: impl Fn(&Value) -> bool) -> Result<String, LlmError> {
    let mut result = None;
    for part in parts {
        if ignored(part) {
            continue;
        }
        let Some(object) = part.as_object() else {
            return Err(invalid_response());
        };
        if object
            .get("thought")
            .is_some_and(|value| value.as_bool() != Some(false))
        {
            return Err(invalid_response());
        }
        if result.is_some()
            || object
                .keys()
                .any(|key| !matches!(key.as_str(), "text" | "thought" | "thoughtSignature"))
        {
            return Err(invalid_response());
        }
        result = part.get("text").and_then(Value::as_str).map(str::to_owned);
        if result.is_none() {
            return Err(invalid_response());
        }
    }
    result.ok_or_else(invalid_response)
}

fn transport_error(error: reqwest::Error) -> LlmError {
    if error.is_timeout() {
        LlmError::new("LLM service request timed out", true)
    } else if error.is_connect() {
        LlmError::new("could not connect to the LLM service", true)
    } else {
        LlmError::new("LLM service request failed", false)
    }
}

fn response_limit_error() -> LlmError {
    LlmError::new("LLM service response exceeds the size limit", false)
}

fn invalid_response() -> LlmError {
    LlmError::new("LLM model returned an invalid decision", false)
}
