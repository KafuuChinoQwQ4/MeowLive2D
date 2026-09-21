//! 提供商协议与资源上限；TOML 只接受密钥环境变量，面板密钥由本机独立文件注入。
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct LlmConfig {
    pub mode: String,
    pub provider: String,
    pub api_format: String,
    pub base_url: String,
    pub model: String,
    pub api_key_env: String,
    #[serde(skip)]
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub max_response_bytes: usize,
    pub max_tokens: u32,
    pub json_mode: bool,
    pub reasoning_effort: String,
    pub max_retries: u32,
}
impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            mode: "cloud".into(),
            provider: "custom".into(),
            api_format: "openai_chat".into(),
            base_url: String::new(),
            model: String::new(),
            api_key_env: "MEOWLIVE_LLM_API_KEY".into(),
            api_key: None,
            timeout_seconds: 30,
            max_response_bytes: 65536,
            max_tokens: 1024,
            json_mode: true,
            reasoning_effort: "default".into(),
            max_retries: 1,
        }
    }
}
impl LlmConfig {
    pub fn is_configured(&self) -> bool {
        !self.base_url.is_empty() && !self.model.is_empty()
    }
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.mode.as_str(), "cloud" | "local") {
            return Err("llm.mode 须为 cloud 或 local".into());
        }
        if !matches!(
            self.api_format.as_str(),
            "openai_chat" | "openai_responses" | "anthropic_messages" | "gemini_generate_content"
        ) {
            return Err("llm.api_format 须为 openai_chat、openai_responses、anthropic_messages 或 gemini_generate_content".into());
        }
        if self.provider.trim().is_empty()
            || self.provider.len() > 64
            || self.provider.chars().any(char::is_control)
        {
            return Err("llm.provider 无效".into());
        }
        if self.mode == "local" {
            let uri = self
                .base_url
                .parse::<axum::http::Uri>()
                .map_err(|_| "本地 LLM 地址无效")?;
            let local = uri.host().is_some_and(|h| {
                h.trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            });
            if !local
                || !self.is_configured()
                || !self.api_key_env.is_empty()
                || self.api_key.is_some()
                || self.max_tokens > 1024
                || self.max_retries != 0
            {
                return Err(
                    "本地预设须使用回环 IP、明确模型、空密钥环境变量，token 不超过 1024 且不重试"
                        .into(),
                );
            }
        }
        if self.base_url.is_empty() != self.model.is_empty() {
            return Err("LLM 地址和模型名须同时填写或同时留空".into());
        }
        if self.base_url.len() > 4096 {
            return Err("LLM 地址过长".into());
        }
        if !self.base_url.is_empty() {
            let uri = self
                .base_url
                .parse::<axum::http::Uri>()
                .map_err(|_| "LLM 地址格式无效")?;
            if !matches!(uri.scheme_str(), Some("http" | "https"))
                || uri.authority().is_none_or(|a| a.as_str().contains('@'))
                || uri.query().is_some()
                || self.base_url.contains('#')
                || self.base_url.chars().any(char::is_whitespace)
            {
                return Err("LLM 地址须为无凭据、查询参数及片段的 HTTP(S) API 根地址".into());
            }
            if self.model.trim().is_empty()
                || self.model.len() > 128
                || self.model.chars().any(char::is_control)
            {
                return Err("LLM 模型名须为 1–128 字节且不含控制字符".into());
            }
        }
        if !self.api_key_env.is_empty()
            && (self.api_key_env.len() > 128
                || !self.api_key_env.bytes().enumerate().all(|(i, c)| {
                    c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                }))
        {
            return Err("api_key_env 须为有效环境变量名或留空".into());
        }
        if self.api_key.as_ref().is_some_and(|key| {
            key.trim().is_empty() || key.len() > 4096 || key.chars().any(char::is_control)
        }) {
            return Err("LLM API key 无效".into());
        }
        if !(1..=120).contains(&self.timeout_seconds)
            || !(1024..=1048576).contains(&self.max_response_bytes)
            || !(64..=65536).contains(&self.max_tokens)
            || self.max_retries > 1
        {
            return Err(
                "LLM 超时须 1–120 秒、响应上限 1 KiB–1 MiB、token 上限 64–65536、重试 0–1 次"
                    .into(),
            );
        }
        let requested = self.reasoning_effort.parse::<meowlive_application::ports::reasoning::ReasoningEffort>()
            .map_err(|_| "llm.reasoning_effort 须为 default、minimal、low、medium、high、xhigh、max 或 ultra")?;
        if self.is_configured() {
            let format = self.api_format.parse().map_err(|_| "LLM API 格式无效")?;
            let resolution = meowlive_adapters::llm::reasoning::resolve_reasoning(
                &self.provider,
                format,
                &self.model,
                requested,
                self.max_tokens,
            );
            if let Some(error) = resolution.error {
                return Err(error);
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for LlmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmConfig")
            .field("provider", &self.provider)
            .field("api_format", &self.api_format)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .finish_non_exhaustive()
    }
}
