//! 有界、可取消的单次模型调用；只向观察者暴露计数和使用量。
mod output;
mod request;
mod stream;

use super::{
    config::{EndpointKind, LlmConfig, ValidatedConfig},
    multi_provider::ApiFormat,
    response::parse_decision_content,
};
use meowlive_application::ports::{
    llm::{DecisionRequest, LlmError},
    llm_runtime::{ModelEvent, ModelOptions, ModelTurn, TokenUsage},
};
use reqwest::{Client, header};
use serde_json::Value;

pub(super) struct RuntimeProvider {
    client: Client,
    config: ValidatedConfig,
    format: ApiFormat,
}
impl RuntimeProvider {
    pub fn new(config: LlmConfig, format: ApiFormat) -> Result<Self, LlmError> {
        let kind = match format {
            ApiFormat::OpenaiChat => EndpointKind::OpenaiChat,
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
            client,
            config,
            format,
        })
    }
    pub async fn turn(
        &self,
        request: DecisionRequest,
        options: ModelOptions,
    ) -> Result<ModelTurn, LlmError> {
        let payload = request::payload(&self.config, self.format, &request, &options)?;
        let mut endpoint = self.config.endpoint.clone();
        if options.stream && self.format == ApiFormat::GeminiGenerateContent {
            let path = endpoint
                .path()
                .strip_suffix(":generateContent")
                .ok_or_else(invalid)?
                .to_owned();
            endpoint.set_path(&format!("{path}:streamGenerateContent"));
            endpoint.set_query(Some("alt=sse"));
        }
        let mut builder = self
            .client
            .post(endpoint)
            .header(
                header::ACCEPT,
                if options.stream {
                    "text/event-stream"
                } else {
                    "application/json"
                },
            )
            .json(&payload);
        if self.format == ApiFormat::AnthropicMessages {
            builder = builder.header("anthropic-version", "2023-06-01");
        }
        if let Some(key) = &self.config.api_key {
            builder = match self.format {
                ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses => {
                    builder.bearer_auth(key.to_str().unwrap_or_default())
                }
                ApiFormat::AnthropicMessages => builder.header("x-api-key", key),
                ApiFormat::GeminiGenerateContent => builder.header("x-goog-api-key", key),
            };
        }
        let mut response = builder.send().await.map_err(transport_error)?;
        let mut progress = Progress::new(&options);
        if !response.status().is_success() {
            let status = response.status();
            // Some compatible gateways attach metering to an HTTP error. Read
            // only a bounded body and never expose the upstream error text.
            if let Ok(raw) = read_json(&mut response, self.config.max_response_bytes).await {
                progress.usage(output::usage(self.format, &raw));
            }
            return Err(LlmError::new(
                format!("LLM service returned HTTP {}", status.as_u16()),
                status.as_u16() == 429 || status.is_server_error(),
            ));
        }
        let response_limit = if options.stream {
            stream::wire_limit(self.config.max_response_bytes)
        } else {
            self.config.max_response_bytes
        };
        if response
            .content_length()
            .is_some_and(|v| v > response_limit as u64)
        {
            return Err(limit());
        }
        let raw = if options.stream {
            stream::receive(
                &mut response,
                self.format,
                self.config.max_response_bytes,
                &mut progress,
            )
            .await?
        } else {
            read_json(&mut response, self.config.max_response_bytes).await?
        };
        // Usage is published before validating content or the finish reason so a
        // paid but refused/truncated response still reaches the call ledger.
        let usage = output::usage(self.format, &raw);
        progress.usage(usage.clone());
        let parsed = output::parse(self.format, &raw, &options.tools)?;
        if options.tool_choice_none && !parsed.calls.is_empty() {
            return Err(invalid());
        }
        if !options.stream {
            progress.output(&parsed.text);
            if !parsed.calls.is_empty() {
                progress.first();
            }
        }
        let decision = if parsed.calls.is_empty() {
            Some(parse_decision_content(&parsed.text, &request)?)
        } else {
            None
        };
        let continuation = (!parsed.calls.is_empty()).then(|| parsed.continuation.to_string());
        Ok(ModelTurn {
            decision,
            tool_calls: parsed.calls,
            continuation,
            usage,
            finish_reason: if parsed.tool_finish {
                "tool_calls"
            } else {
                "stop"
            }
            .into(),
        })
    }
}

async fn read_json(response: &mut reqwest::Response, max_bytes: usize) -> Result<Value, LlmError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(limit());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if bytes.len().saturating_add(chunk.len()) > max_bytes {
            return Err(limit());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}

struct Progress<'a> {
    options: &'a ModelOptions,
    first: bool,
    characters: u64,
    last_usage: TokenUsage,
}
impl<'a> Progress<'a> {
    fn new(options: &'a ModelOptions) -> Self {
        Self {
            options,
            first: false,
            characters: 0,
            last_usage: TokenUsage::default(),
        }
    }
    fn emit(&self, event: ModelEvent) {
        if let Some(observer) = &self.options.observer {
            observer.on_event(event);
        }
    }
    fn first(&mut self) {
        if !self.first {
            self.first = true;
            self.emit(ModelEvent::FirstToken);
        }
    }
    fn output(&mut self, text: &str) {
        if !text.is_empty() {
            self.first();
            self.characters = self.characters.saturating_add(text.chars().count() as u64);
            self.emit(ModelEvent::OutputProgress {
                characters: self.characters,
            });
        }
    }
    fn usage(&mut self, usage: TokenUsage) {
        if usage != TokenUsage::default() && usage != self.last_usage {
            self.last_usage = usage.clone();
            self.emit(ModelEvent::Usage(usage));
        }
    }
}
fn invalid() -> LlmError {
    LlmError::new("LLM model returned an invalid or incomplete turn", false)
}
fn limit() -> LlmError {
    LlmError::new("LLM service response exceeds the size limit", false)
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
