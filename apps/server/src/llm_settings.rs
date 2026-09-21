//! 本机 LLM 配置持久化；保存仅影响下次启动，密钥不进入查询 DTO。
use crate::config::LlmConfig;
use meowlive_adapters::llm::{
    models::{ModelCatalogConfig, canonical_base_url},
    multi_provider::ApiFormat,
};
use meowlive_protocol::llm::{
    LlmModelsRequest, LlmSettings, LlmSettingsRequest, LlmSettingsSnapshot,
};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSettings {
    schema: u8,
    config: LlmConfig,
    api_key: Option<String>,
}

pub struct LlmSettingsStore {
    active: LlmConfig,
    path: Option<PathBuf>,
    edits: Mutex<()>,
    pub(crate) tests: tokio::sync::Semaphore,
}

pub fn settings_path(config_path: &Path) -> PathBuf {
    let name = config_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    config_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("local")
        .join(format!("{name}-llm.json"))
}

pub fn load_override(config_path: &Path) -> Result<Option<LlmConfig>, String> {
    read_saved(&settings_path(config_path))
}

fn read_saved(path: &Path) -> Result<Option<LlmConfig>, String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("无法读取本机 LLM 配置".into()),
    };
    if !metadata.is_file() || metadata.len() > 16384 {
        return Err("本机 LLM 配置文件无效".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "无法读取本机 LLM 配置")?;
    let saved: SavedSettings =
        serde_json::from_slice(&bytes).map_err(|_| "本机 LLM 配置格式无效")?;
    if saved.schema != 1 {
        return Err("本机 LLM 配置版本无效".into());
    }
    let mut config = saved.config;
    config.api_key = saved.api_key;
    config.validate()?;
    Ok(Some(config))
}

impl LlmSettingsStore {
    pub fn new(active: LlmConfig, path: Option<PathBuf>) -> Self {
        Self {
            active,
            path,
            edits: Mutex::new(()),
            tests: tokio::sync::Semaphore::new(1),
        }
    }
    fn current(&self) -> Result<LlmConfig, String> {
        match &self.path {
            Some(path) => Ok(read_saved(path)?.unwrap_or_else(|| self.active.clone())),
            None => Ok(self.active.clone()),
        }
    }
    pub async fn snapshot(&self) -> Result<LlmSettingsSnapshot, String> {
        let _edit = self.edits.lock().await;
        let config = self.current()?;
        Ok(self.snapshot_for(&config))
    }
    fn snapshot_for(&self, config: &LlmConfig) -> LlmSettingsSnapshot {
        LlmSettingsSnapshot {
            settings: public_settings(config),
            key_configured: config.api_key.is_some()
                || (!config.api_key_env.is_empty()
                    && std::env::var(&config.api_key_env).is_ok_and(|key| !key.trim().is_empty())),
            restart_required: config != &self.active,
            active_model: if crate::bootstrap::build_model(&self.active)
                .is_ok_and(|model| model.is_some())
            {
                self.active.model.clone()
            } else {
                String::new()
            },
            storage_available: self.path.is_some(),
        }
    }
    fn resolve(&self, request: LlmSettingsRequest) -> Result<LlmConfig, String> {
        let old = self.current()?;
        let settings = request.settings;
        let api_key = resolve_key(
            &old,
            &settings.provider,
            &settings.api_format,
            &settings.base_url,
            request.api_key,
            request.clear_api_key,
        )?;
        let config = LlmConfig {
            provider: settings.provider,
            api_format: settings.api_format,
            base_url: settings.base_url,
            model: settings.model,
            mode: settings.mode,
            timeout_seconds: u64::from(settings.timeout_seconds),
            max_tokens: settings.max_tokens,
            json_mode: settings.json_mode,
            reasoning_effort: settings.reasoning_effort,
            api_key,
            api_key_env: String::new(),
            max_retries: if old.mode == "local" {
                0
            } else {
                old.max_retries
            },
            max_response_bytes: old.max_response_bytes,
        };
        let mut config = config;
        if config.mode == "local" {
            config.max_retries = 0;
        }
        config.validate()?;
        if !config.is_configured() {
            return Err("请填写 API 地址并选择模型".into());
        }
        // Validate headers and adapter configuration before persisting any change.
        crate::bootstrap::build_model(&config)?;
        Ok(config)
    }
    pub async fn candidate(&self, request: LlmSettingsRequest) -> Result<LlmConfig, String> {
        let _edit = self.edits.lock().await;
        self.resolve(request)
    }
    pub async fn models_candidate(
        &self,
        request: LlmModelsRequest,
    ) -> Result<ModelCatalogConfig, String> {
        let _edit = self.edits.lock().await;
        if request.provider.trim().is_empty()
            || request.provider.len() > 64
            || request.provider.chars().any(char::is_control)
        {
            return Err("LLM 服务商标识无效".into());
        }
        if !matches!(request.mode.as_str(), "cloud" | "local") {
            return Err("LLM 运行模式无效".into());
        }
        let format = request
            .api_format
            .parse::<ApiFormat>()
            .map_err(|_| "LLM API 格式无效")?;
        let canonical =
            canonical_base_url(&request.base_url, format).map_err(|error| error.message)?;
        let supplied_key = request.api_key.is_some();
        let api_key = resolve_key(
            &self.current()?,
            &request.provider,
            &request.api_format,
            &request.base_url,
            request.api_key,
            request.clear_api_key,
        )?;
        if request.mode == "local" {
            let local = request
                .base_url
                .parse::<axum::http::Uri>()
                .ok()
                .and_then(|uri| uri.host().map(str::to_owned))
                .is_some_and(|host| {
                    host.trim_matches(['[', ']'])
                        .parse::<std::net::IpAddr>()
                        .is_ok_and(|address| address.is_loopback())
                });
            if !local || api_key.is_some() {
                return Err("本地模型列表须使用回环 IP 地址且不携带 API 密钥".into());
            }
        }
        Ok(ModelCatalogConfig {
            // Reusing a saved credential must stay inside its original API prefix.
            // An explicit models endpoint disables discovery's version probes.
            base_url: if !supplied_key && api_key.is_some() {
                format!("{canonical}/models")
            } else {
                request.base_url
            },
            api_key,
            api_format: format,
            timeout: std::time::Duration::from_secs(30),
        })
    }
    pub async fn save(&self, request: LlmSettingsRequest) -> Result<LlmSettingsSnapshot, String> {
        let _edit = self.edits.lock().await;
        let path = self.path.as_ref().ok_or("此主服务未配置本机配置存储")?;
        let config = self.resolve(request)?;
        let parent = path.parent().ok_or("本机配置目录无效")?;
        std::fs::create_dir_all(parent).map_err(|_| "无法建立本机配置目录")?;
        if !std::fs::symlink_metadata(parent).is_ok_and(|m| m.is_dir()) {
            return Err("本机配置目录无效".into());
        }
        let saved = SavedSettings {
            schema: 1,
            config: config.clone(),
            api_key: config.api_key.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&saved).map_err(|_| "无法保存 LLM 配置")?;
        if bytes.len() > 16384 {
            return Err("LLM 配置总长度超过上限".into());
        }
        let temporary = parent.join(format!(".llm-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
            return Err("无法保存本机 LLM 配置，原配置保持不变".into());
        }
        Ok(self.snapshot_for(&config))
    }
}

fn resolve_key(
    old: &LlmConfig,
    provider: &str,
    format: &str,
    base_url: &str,
    api_key: Option<String>,
    clear_api_key: bool,
) -> Result<Option<String>, String> {
    if clear_api_key && api_key.is_some() {
        return Err("不能同时提交和清除密钥".into());
    }
    let same_destination = old.provider == provider
        && old.api_format == format
        && format.parse::<ApiFormat>().ok().is_some_and(|kind| {
            match (
                canonical_base_url(&old.base_url, kind),
                canonical_base_url(base_url, kind),
            ) {
                (Ok(old), Ok(new)) => old == new,
                _ => false,
            }
        });
    let key = if clear_api_key {
        None
    } else if api_key.is_some() {
        api_key
    } else if same_destination {
        old.api_key.clone().or_else(|| {
            (!old.api_key_env.is_empty())
                .then(|| std::env::var(&old.api_key_env).ok())
                .flatten()
        })
    } else {
        None
    };
    if key.as_ref().is_some_and(|key| {
        key.trim().is_empty() || key.len() > 4096 || key.chars().any(char::is_control)
    }) {
        return Err("LLM API 密钥无效".into());
    }
    Ok(key)
}

fn public_settings(config: &LlmConfig) -> LlmSettings {
    LlmSettings {
        provider: config.provider.clone(),
        api_format: config.api_format.clone(),
        base_url: config.base_url.clone(),
        model: config.model.clone(),
        mode: config.mode.clone(),
        timeout_seconds: config.timeout_seconds as u32,
        max_tokens: config.max_tokens,
        json_mode: config.json_mode,
        reasoning_effort: config.reasoning_effort.clone(),
    }
}
