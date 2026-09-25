//! 本机 LLM 配置持久化；保存仅影响下次启动，密钥不进入查询 DTO。
use crate::config::LlmConfig;
use meowlive_adapters::llm::{
    models::{ModelCatalogConfig, canonical_base_url},
    multi_provider::ApiFormat,
};
use meowlive_protocol::llm::{
    LlmModelsRequest, LlmProfileCreateRequest, LlmProfileIdRequest, LlmProfileRenameRequest,
    LlmProfileSummary, LlmSettings, LlmSettingsRequest, LlmSettingsSnapshot,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io::Write,
    path::{Path, PathBuf},
};
use tokio::sync::Mutex;

const MAX_PROFILES: usize = 32;
const MAX_PROFILE_NAME_CHARS: usize = 64;
const MAX_FILE_BYTES: u64 = 128 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedProfile {
    id: String,
    name: String,
    config: LlmConfig,
    api_key: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSettings {
    schema: u8,
    selected_profile_id: Option<String>,
    profiles: Vec<SavedProfile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySavedSettings {
    schema: u8,
    config: LlmConfig,
    api_key: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SavedFile {
    Current(SavedSettings),
    Legacy(LegacySavedSettings),
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
    Ok(read_saved(&settings_path(config_path))?.and_then(|saved| {
        saved
            .selected_profile_id
            .as_ref()
            .and_then(|id| saved.profiles.iter().find(|profile| &profile.id == id))
            .map(profile_config)
    }))
}

fn empty_saved() -> SavedSettings {
    SavedSettings {
        schema: 2,
        selected_profile_id: None,
        profiles: Vec::new(),
    }
}

fn read_saved(path: &Path) -> Result<Option<SavedSettings>, String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("无法读取本机 LLM 配置".into()),
    };
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err("本机 LLM 配置文件无效".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "无法读取本机 LLM 配置")?;
    let saved = match serde_json::from_slice::<SavedFile>(&bytes)
        .map_err(|_| "本机 LLM 配置格式无效")?
    {
        SavedFile::Current(saved) => {
            if saved.schema != 2 {
                return Err("本机 LLM 配置版本无效".into());
            }
            saved
        }
        SavedFile::Legacy(saved) => {
            if saved.schema != 1 {
                return Err("本机 LLM 配置版本无效".into());
            }
            let mut config = saved.config;
            config.api_key = None;
            SavedSettings {
                schema: 2,
                selected_profile_id: Some("legacy".into()),
                profiles: vec![SavedProfile {
                    id: "legacy".into(),
                    name: "配置1".into(),
                    config,
                    api_key: saved.api_key,
                }],
            }
        }
    };
    validate_saved(&saved)?;
    Ok(Some(saved))
}

fn validate_saved(saved: &SavedSettings) -> Result<(), String> {
    if saved.profiles.len() > MAX_PROFILES {
        return Err("本机 LLM 配置数量超过上限".into());
    }
    let mut ids = HashSet::new();
    for profile in &saved.profiles {
        if profile.id.trim().is_empty()
            || profile.id.len() > 64
            || profile.id.chars().any(char::is_control)
            || !ids.insert(&profile.id)
        {
            return Err("本机 LLM 配置标识无效".into());
        }
        validate_profile_name(&profile.name)?;
        let mut config = profile.config.clone();
        config.api_key = profile.api_key.clone();
        config.validate()?;
    }
    if saved
        .selected_profile_id
        .as_ref()
        .is_some_and(|id| !saved.profiles.iter().any(|profile| &profile.id == id))
    {
        return Err("本机 LLM 当前配置标识无效".into());
    }
    Ok(())
}

fn validate_profile_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || name.chars().count() > MAX_PROFILE_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return Err("LLM 配置标题须为 1–64 个字符且不含控制字符".into());
    }
    Ok(())
}

fn next_profile_name(saved: &SavedSettings) -> String {
    (1..=MAX_PROFILES + 1)
        .map(|number| format!("配置{number}"))
        .find(|name| !saved.profiles.iter().any(|profile| profile.name == *name))
        .unwrap_or_else(|| format!("配置{}", saved.profiles.len() + 1))
}

fn profile_config(profile: &SavedProfile) -> LlmConfig {
    let mut config = profile.config.clone();
    config.api_key = profile.api_key.clone();
    config
}

fn selected_config(saved: &SavedSettings) -> Option<LlmConfig> {
    saved
        .selected_profile_id
        .as_ref()
        .and_then(|id| saved.profiles.iter().find(|profile| &profile.id == id))
        .map(profile_config)
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
            Some(path) => Ok(read_saved(path)?
                .as_ref()
                .and_then(selected_config)
                .unwrap_or_else(|| self.active.clone())),
            None => Ok(self.active.clone()),
        }
    }
    pub async fn snapshot(&self) -> Result<LlmSettingsSnapshot, String> {
        let _edit = self.edits.lock().await;
        let saved = self.saved()?;
        let config = saved
            .as_ref()
            .and_then(selected_config)
            .unwrap_or_else(|| self.active.clone());
        Ok(self.snapshot_for(&config, saved.as_ref()))
    }
    fn saved(&self) -> Result<Option<SavedSettings>, String> {
        match &self.path {
            Some(path) => read_saved(path),
            None => Ok(None),
        }
    }
    fn snapshot_for(
        &self,
        config: &LlmConfig,
        saved: Option<&SavedSettings>,
    ) -> LlmSettingsSnapshot {
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
            profiles: saved
                .map(|saved| {
                    saved
                        .profiles
                        .iter()
                        .map(|profile| {
                            let config = profile_config(profile);
                            LlmProfileSummary {
                                id: profile.id.clone(),
                                name: profile.name.clone(),
                                settings: public_settings(&config),
                                key_configured: config.api_key.is_some(),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            selected_profile_id: saved.and_then(|saved| saved.selected_profile_id.clone()),
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
        let config = config_from_settings(&old, settings, api_key)?;
        // Validate headers and adapter configuration before persisting any change.
        crate::bootstrap::build_model(&config)?;
        Ok(config)
    }
    pub async fn candidate(&self, request: LlmSettingsRequest) -> Result<LlmConfig, String> {
        let _edit = self.edits.lock().await;
        self.resolve(request)
    }
    fn resolve_profile_create(
        &self,
        request: &LlmProfileCreateRequest,
    ) -> Result<LlmConfig, String> {
        let old = self.current()?;
        let api_key = resolve_key(
            &old,
            &request.settings.provider,
            &request.settings.api_format,
            &request.settings.base_url,
            request.api_key.clone(),
            request.clear_api_key,
        )?;
        let config = config_from_settings(&old, request.settings.clone(), api_key)?;
        crate::bootstrap::build_model(&config)?;
        Ok(config)
    }
    pub async fn profile_candidate(
        &self,
        request: LlmProfileCreateRequest,
    ) -> Result<LlmConfig, String> {
        let _edit = self.edits.lock().await;
        self.resolve_profile_create(&request)
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
        self.path.as_ref().ok_or("此主服务未配置本机配置存储")?;
        let mut saved = self.saved()?.unwrap_or_else(empty_saved);
        let config = self.resolve(request)?;
        let id = if let Some(id) = saved.selected_profile_id.clone() {
            let profile = saved
                .profiles
                .iter_mut()
                .find(|profile| profile.id == id)
                .ok_or("本机 LLM 当前配置不存在")?;
            profile.config = config_without_key(&config);
            profile.api_key = config.api_key.clone();
            id
        } else {
            if saved.profiles.len() >= MAX_PROFILES {
                return Err("本机 LLM 配置数量超过上限".into());
            }
            let id = uuid::Uuid::new_v4().to_string();
            let name = next_profile_name(&saved);
            saved.profiles.push(SavedProfile {
                id: id.clone(),
                name,
                config: config_without_key(&config),
                api_key: config.api_key.clone(),
            });
            id
        };
        saved.selected_profile_id = Some(id);
        self.persist(&saved)?;
        Ok(self.snapshot_for(&config, Some(&saved)))
    }
    pub async fn create_profile(
        &self,
        request: LlmProfileCreateRequest,
    ) -> Result<LlmSettingsSnapshot, String> {
        let _edit = self.edits.lock().await;
        self.path.as_ref().ok_or("此主服务未配置本机配置存储")?;
        let mut saved = self.saved()?.unwrap_or_else(empty_saved);
        if saved.profiles.len() >= MAX_PROFILES {
            return Err("本机 LLM 配置数量超过上限".into());
        }
        let name = if request.name.trim().is_empty() {
            next_profile_name(&saved)
        } else {
            request.name.trim().to_owned()
        };
        validate_profile_name(&name)?;
        let config = self.resolve_profile_create(&request)?;
        let id = uuid::Uuid::new_v4().to_string();
        saved.profiles.push(SavedProfile {
            id: id.clone(),
            name,
            config: config_without_key(&config),
            api_key: config.api_key.clone(),
        });
        saved.selected_profile_id = Some(id);
        self.persist(&saved)?;
        Ok(self.snapshot_for(&config, Some(&saved)))
    }
    pub async fn select_profile(
        &self,
        request: LlmProfileIdRequest,
    ) -> Result<LlmSettingsSnapshot, String> {
        let _edit = self.edits.lock().await;
        self.path.as_ref().ok_or("此主服务未配置本机配置存储")?;
        let mut saved = self.saved()?.unwrap_or_else(empty_saved);
        if !saved
            .profiles
            .iter()
            .any(|profile| profile.id == request.id)
        {
            return Err("要切换的 LLM 配置不存在".into());
        }
        saved.selected_profile_id = Some(request.id);
        let config = selected_config(&saved).ok_or("要切换的 LLM 配置不存在")?;
        self.persist(&saved)?;
        Ok(self.snapshot_for(&config, Some(&saved)))
    }
    pub async fn rename_profile(
        &self,
        request: LlmProfileRenameRequest,
    ) -> Result<LlmSettingsSnapshot, String> {
        let _edit = self.edits.lock().await;
        self.path.as_ref().ok_or("此主服务未配置本机配置存储")?;
        let name = request.name.trim().to_owned();
        validate_profile_name(&name)?;
        let mut saved = self.saved()?.unwrap_or_else(empty_saved);
        let profile = saved
            .profiles
            .iter_mut()
            .find(|profile| profile.id == request.id)
            .ok_or("要重命名的 LLM 配置不存在")?;
        profile.name = name;
        let config = selected_config(&saved).unwrap_or_else(|| self.active.clone());
        self.persist(&saved)?;
        Ok(self.snapshot_for(&config, Some(&saved)))
    }
    pub async fn delete_profile(
        &self,
        request: LlmProfileIdRequest,
    ) -> Result<LlmSettingsSnapshot, String> {
        let _edit = self.edits.lock().await;
        self.path.as_ref().ok_or("此主服务未配置本机配置存储")?;
        let mut saved = self.saved()?.unwrap_or_else(empty_saved);
        let position = saved
            .profiles
            .iter()
            .position(|profile| profile.id == request.id)
            .ok_or("要删除的 LLM 配置不存在")?;
        let deleting_selected = saved.selected_profile_id.as_deref() == Some(&request.id);
        saved.profiles.remove(position);
        if deleting_selected {
            saved.selected_profile_id = saved.profiles.first().map(|profile| profile.id.clone());
        }
        let config = selected_config(&saved).unwrap_or_else(|| self.active.clone());
        self.persist(&saved)?;
        Ok(self.snapshot_for(&config, Some(&saved)))
    }
    fn persist(&self, saved: &SavedSettings) -> Result<(), String> {
        validate_saved(saved)?;
        let path = self.path.as_ref().ok_or("此主服务未配置本机配置存储")?;
        let parent = path.parent().ok_or("本机配置目录无效")?;
        std::fs::create_dir_all(parent).map_err(|_| "无法建立本机配置目录")?;
        if !std::fs::symlink_metadata(parent).is_ok_and(|m| m.is_dir()) {
            return Err("本机配置目录无效".into());
        }
        let bytes = serde_json::to_vec_pretty(saved).map_err(|_| "无法保存 LLM 配置")?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
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
        Ok(())
    }
}

fn config_without_key(config: &LlmConfig) -> LlmConfig {
    let mut config = config.clone();
    config.api_key = None;
    config
}

fn config_from_settings(
    old: &LlmConfig,
    settings: LlmSettings,
    api_key: Option<String>,
) -> Result<LlmConfig, String> {
    let mut config = LlmConfig {
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
    if config.mode == "local" {
        config.max_retries = 0;
    }
    config.validate()?;
    if !config.is_configured() {
        return Err("请填写 API 地址并选择模型".into());
    }
    Ok(config)
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
