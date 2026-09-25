//! 显式 TOML 配置与启动前校验；不把机器路径发送给面板。
use serde::Deserialize;
use std::{net::SocketAddr, path::Path};

mod agent;
mod auth;
mod graph;
mod live;
mod llm;
mod memory;
mod resources;
mod training;
mod viewers;
pub use agent::AgentConfig;
pub use auth::AuthConfig;
pub use graph::GraphConfig;
pub use live::{LiveConfig, LiveCredentials};
pub use llm::LlmConfig;
pub use memory::MemoryConfig;
pub use resources::ResourcesConfig;
pub use training::TrainingConfig;
pub use viewers::ViewersConfig;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub speech: SpeechConfig,
    pub agent: AgentConfig,
    pub llm: LlmConfig,
    pub live: LiveConfig,
    pub resources: ResourcesConfig,
    pub training: TrainingConfig,
    pub viewers: ViewersConfig,
    pub memory: MemoryConfig,
    pub graph: GraphConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub listen_address: String,
    pub queue_capacity: usize,
    pub history_limit: usize,
    pub allowed_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1:19600".into(),
            queue_capacity: 8,
            history_limit: 50,
            allowed_origins: vec![
                "http://127.0.0.1:1420".into(),
                "http://localhost:1420".into(),
                "http://tauri.localhost".into(),
            ],
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpeechConfig {
    pub base_url: String,
    pub reference_audio: String,
    pub prompt_text: String,
    pub prompt_language: String,
    pub text_language: String,
    pub max_concurrency: usize,
    pub timeout_seconds: u64,
    pub max_audio_bytes: usize,
}

impl Default for SpeechConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:9880".into(),
            reference_audio: String::new(),
            prompt_text: String::new(),
            prompt_language: "zh".into(),
            text_language: "zh".into(),
            max_concurrency: 1,
            timeout_seconds: 300,
            max_audio_bytes: 32 * 1024 * 1024,
        }
    }
}

#[cfg(test)]
mod speech_limits_tests {
    use super::AppConfig;

    #[test]
    fn default_limits_allow_a_four_minute_32khz_mono_wav() {
        let config = AppConfig::default();
        let wav_bytes = 44 + 32_000 * 240 * 2;
        assert!(config.speech.max_audio_bytes >= wav_bytes);
        assert_eq!(config.speech.timeout_seconds, 300);
    }

    #[test]
    fn speech_configuration_accepts_32_mib_and_rejects_larger_wav_limits() {
        assert!(AppConfig::parse("[speech]\nmax_audio_bytes=33554432").is_ok());
        assert!(AppConfig::parse("[speech]\nmax_audio_bytes=33554433").is_err());
    }
}

impl AppConfig {
    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self =
            toml::from_str(text).map_err(|_| "配置格式错误，请检查字段名称、类型和 TOML 语法")?;
        config.validate()?;
        Ok(config)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("无法读取配置：{e}"))?;
        let mut config = Self::parse(&text)?;
        if let Some(llm) = crate::llm_settings::load_override(path)? {
            config.llm = llm;
        }
        if let Some(live) = crate::live_settings::load_override(path, &config.live)? {
            config.live = live;
        }
        if let Some(agent) = crate::agent_settings::load_override(path)? {
            config.agent.persona = agent.persona;
            config.agent.system_prompt = agent.system_prompt;
            config.agent.topic = agent.topic;
            config.agent.proactive_enabled = agent.proactive_enabled;
            config.agent.cooldown_ms = agent.cooldown_ms;
            config.agent.interaction = crate::agent::mapping::interaction_dto(agent.interaction);
        }
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.auth.validate()?;
        self.agent.validate()?;
        self.llm.validate()?;
        self.live.validate()?;
        self.resources.validate()?;
        self.training.validate()?;
        self.viewers.validate()?;
        self.memory.validate(self.viewers.enabled)?;
        self.graph.validate(self.viewers.enabled)?;
        if self.llm.mode == "local" {
            let uri = self
                .speech
                .base_url
                .parse::<axum::http::Uri>()
                .map_err(|_| "本地预设 TTS 地址无效")?;
            if !uri.host().is_some_and(|h| {
                h.trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            }) {
                return Err("离线预设的 TTS 也必须使用回环 IP".into());
            }
        }
        self.server
            .listen_address
            .parse::<SocketAddr>()
            .map_err(|_| "listen_address 必须是 IP:端口")?;
        if !(1..=64).contains(&self.server.queue_capacity)
            || !(1..=1000).contains(&self.server.history_limit)
        {
            return Err("queue_capacity 必须在 1..64，history_limit 必须在 1..1000".into());
        }
        if self.speech.max_concurrency != 1
            || !(1..=300).contains(&self.speech.timeout_seconds)
            || !(1024..=32 * 1024 * 1024).contains(&self.speech.max_audio_bytes)
        {
            return Err("首版语音并发必须为 1，超时为 1..300 秒，WAV 上限为 1 KiB..32 MiB".into());
        }
        for origin in &self.server.allowed_origins {
            let uri = origin
                .parse::<axum::http::Uri>()
                .map_err(|_| "allowed_origins 无效")?;
            if !matches!(uri.scheme_str(), Some("http" | "https"))
                || uri.authority().is_none()
                || origin.parse::<axum::http::HeaderValue>().is_err()
            {
                return Err("allowed_origins 必须为 HTTP(S) 来源".into());
            }
        }
        Ok(())
    }
}
