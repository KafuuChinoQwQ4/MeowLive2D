//! 兼容非流式 Chat Completions 的受限结构化决策适配器。

use meowlive_application::ports::llm::{DecisionFuture, DecisionRequest, LanguageModel, LlmError};
use reqwest::{Client, header};

pub use super::config::LlmConfig;
use super::{config::ValidatedConfig, prompt::build_payload, response::parse_decision};

pub struct OpenAiCompatible {
    client: Client,
    config: ValidatedConfig,
}

impl OpenAiCompatible {
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let config = ValidatedConfig::new(config)?;
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.timeout)
            .connect_timeout(config.timeout)
            .build()
            .map_err(|_| LlmError::new("could not initialize the LLM HTTP client", false))?;
        Ok(Self { client, config })
    }

    async fn request_decision(
        &self,
        request: DecisionRequest,
    ) -> Result<meowlive_application::ports::llm::AgentDecision, LlmError> {
        let payload = build_payload(&request, &self.config)?;
        let mut builder = self
            .client
            .post(self.config.endpoint.clone())
            .header(header::ACCEPT, "application/json")
            .json(&payload);
        if let Some(api_key) = &self.config.api_key {
            builder = builder.header(header::AUTHORIZATION, api_key);
        }
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
        parse_decision(&bytes, &request)
    }
}

impl LanguageModel for OpenAiCompatible {
    fn decide(&self, request: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(self.request_decision(request))
    }
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
