//! 提供商地址与资源上限；只登记密钥环境变量名，不接受密钥明文。
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LlmConfig {
    pub mode: String,
    pub base_url: String,
    pub model: String,
    pub api_key_env: String,
    pub timeout_seconds: u64,
    pub max_response_bytes: usize,
    pub max_tokens: u32,
    pub json_mode: bool,
    pub max_retries: u32,
}
impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            mode: "cloud".into(),
            base_url: String::new(),
            model: String::new(),
            api_key_env: "MEOWLIVE_LLM_API_KEY".into(),
            timeout_seconds: 30,
            max_response_bytes: 65536,
            max_tokens: 1024,
            json_mode: true,
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
        if !(1..=120).contains(&self.timeout_seconds)
            || !(1024..=1048576).contains(&self.max_response_bytes)
            || !(64..=4096).contains(&self.max_tokens)
            || self.max_retries > 1
        {
            return Err(
                "LLM 超时须 1–120 秒、响应上限 1 KiB–1 MiB、token 上限 64–4096、重试 0–1 次".into(),
            );
        }
        Ok(())
    }
}
