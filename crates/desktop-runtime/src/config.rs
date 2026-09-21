//! 独立桌面客户端的显式 TOML 配置与资源上限校验。

use crate::{
    audio::buffer::MAX_INPUT_SAMPLES, avatar::VtsConfig, lip_sync::LipSyncConfig, obs::ObsConfig,
};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClientConfig {
    pub server_url: String,
    pub device_token_file: Option<std::path::PathBuf>,
    pub model_directory: Option<std::path::PathBuf>,
    pub max_buffer_samples: usize,
    pub handshake_timeout_ms: u64,
    pub reconnect_delay_ms: u64,
    pub vtube_studio: VtsConfig,
    pub lip_sync: LipSyncConfig,
    pub obs: ObsConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            model_directory: None,
            device_token_file: None,
            server_url: "http://127.0.0.1:19600".into(),
            max_buffer_samples: MAX_INPUT_SAMPLES,
            handshake_timeout_ms: 5_000,
            reconnect_delay_ms: 2_000,
            vtube_studio: VtsConfig::default(),
            lip_sync: LipSyncConfig::default(),
            obs: ObsConfig::default(),
        }
    }
}

impl ClientConfig {
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|error| error.to_string())?;
        config.vtube_studio.validate()?;
        if config
            .device_token_file
            .as_ref()
            .is_some_and(|p| p.as_os_str().is_empty())
        {
            return Err("device_token_file must name a private credential file".into());
        }
        if config
            .model_directory
            .as_ref()
            .is_some_and(|p| p.as_os_str().is_empty())
        {
            return Err("model_directory must name the VTS Live2DModels directory".into());
        }
        config.lip_sync.validate()?;
        config.obs.validate()?;
        let url = url::Url::parse(&config.server_url).map_err(|error| error.to_string())?;
        if !matches!(url.scheme(), "http" | "ws")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            return Err(
                "server_url must be an http:// or ws:// origin without credentials, path or query"
                    .into(),
            );
        }
        if !(1..=MAX_INPUT_SAMPLES).contains(&config.max_buffer_samples)
            || !(100..=60_000).contains(&config.handshake_timeout_ms)
            || !(100..=60_000).contains(&config.reconnect_delay_ms)
        {
            return Err("invalid desktop buffer or connection timeout limits".into());
        }
        Ok(config)
    }

    pub fn resolve_paths(&mut self, config_path: &std::path::Path) {
        self.obs.settings_path = Some(config_path.with_extension("obs.local.json"));
        for path in [&mut self.model_directory, &mut self.device_token_file]
            .into_iter()
            .flatten()
        {
            if path.is_relative() {
                *path = config_path
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join(&*path);
            }
        }
        if self.vtube_studio.token_path.is_relative() {
            self.vtube_studio.token_path = config_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(&self.vtube_studio.token_path);
        }
    }
}
