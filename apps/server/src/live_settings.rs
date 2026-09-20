//! 直播凭据本机覆盖文件：有界读取、脱敏查询和原子替换。
use crate::config::{LiveConfig, LiveCredentials};
use meowlive_protocol::live::{LiveSettingsRequest, LiveSettingsSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 16_384;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSettings {
    schema: u8,
    enabled: bool,
    app_id: u64,
    credentials: LiveCredentials,
}

#[derive(Default)]
pub struct LiveSettingsStore {
    path: Option<PathBuf>,
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
        .join(format!("{name}-live.json"))
}

pub fn load_override(config_path: &Path, base: &LiveConfig) -> Result<Option<LiveConfig>, String> {
    let path = settings_path(config_path);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("无法读取已保存的直播设置".into()),
    };
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err("已保存的直播设置文件无效".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "无法读取已保存的直播设置")?;
    let saved: SavedSettings =
        serde_json::from_slice(&bytes).map_err(|_| "已保存的直播设置格式无效")?;
    if saved.schema != 1 {
        return Err("已保存的直播设置版本无效".into());
    }
    if saved.enabled && !saved.credentials.complete() {
        return Err("已保存的直播凭据不完整".into());
    }
    let config = LiveConfig {
        enabled: saved.enabled,
        app_id: saved.app_id,
        credentials: Some(saved.credentials),
        ..base.clone()
    };
    config.validate()?;
    Ok(Some(config))
}

impl LiveSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    pub fn snapshot(&self, config: &LiveConfig) -> LiveSettingsSnapshot {
        let credentials = config.resolved_credentials();
        LiveSettingsSnapshot {
            enabled: config.enabled,
            app_id: if config.app_id == 0 {
                String::new()
            } else {
                config.app_id.to_string()
            },
            access_key_id_configured: credentials.access_key_id.is_some(),
            access_key_secret_configured: credentials.access_key_secret.is_some(),
            identity_code_configured: credentials.identity_code.is_some(),
            storage_available: self.path.is_some(),
        }
    }

    pub fn candidate(
        &self,
        current: &LiveConfig,
        request: LiveSettingsRequest,
    ) -> Result<LiveConfig, String> {
        let id = request.app_id.trim();
        let app_id = if id.is_empty() && !request.enabled {
            0
        } else {
            if id.is_empty() || !id.bytes().all(|c| c.is_ascii_digit()) {
                return Err("应用 ID 须为有效的数字".into());
            }
            id.parse::<u64>().map_err(|_| "应用 ID 超过支持范围")?
        };
        if app_id > i64::MAX as u64 {
            return Err("应用 ID 超过支持范围".into());
        }
        let normalize = |value: Option<String>| {
            value
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let supplied = LiveCredentials {
            access_key_id: normalize(request.access_key_id),
            access_key_secret: normalize(request.access_key_secret),
            identity_code: normalize(request.identity_code),
        };
        if request.clear_credentials
            && (request.enabled
                || supplied.access_key_id.is_some()
                || supplied.access_key_secret.is_some()
                || supplied.identity_code.is_some())
        {
            return Err("清除凭据时请关闭直播接入，并保持凭据输入框为空".into());
        }
        let old = if request.clear_credentials || current.app_id != app_id {
            LiveCredentials::default()
        } else {
            current.resolved_credentials()
        };
        let credentials = LiveCredentials {
            access_key_id: supplied.access_key_id.or(old.access_key_id),
            access_key_secret: supplied.access_key_secret.or(old.access_key_secret),
            identity_code: supplied.identity_code.or(old.identity_code),
        };
        credentials.validate()?;
        if request.enabled && !credentials.complete() {
            return Err("请填写开发者 AccessKey ID、AccessKey Secret 和主播身份码；更换应用 ID 后须重新填写凭据".into());
        }
        let config = LiveConfig {
            enabled: request.enabled,
            app_id,
            credentials: Some(credentials),
            ..current.clone()
        };
        config.validate()?;
        Ok(config)
    }

    // Caller holds the live state lock through file replacement and source update.
    pub fn save(&self, config: &LiveConfig) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or("当前主服务不支持保存本机直播设置")?;
        let parent = path.parent().ok_or("本机直播设置目录无效")?;
        std::fs::create_dir_all(parent).map_err(|_| "无法建立本机直播设置目录")?;
        if !std::fs::symlink_metadata(parent).is_ok_and(|metadata| metadata.is_dir()) {
            return Err("本机直播设置目录无效".into());
        }
        let saved = SavedSettings {
            schema: 1,
            enabled: config.enabled,
            app_id: config.app_id,
            credentials: config.resolved_credentials(),
        };
        let bytes = serde_json::to_vec_pretty(&saved).map_err(|_| "无法保存直播设置")?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("直播设置超过长度上限".into());
        }
        let temporary = parent.join(format!(".live-{}.tmp", uuid::Uuid::new_v4()));
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
            drop(file);
            std::fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
            return Err("无法保存本机直播设置，原配置保持不变".into());
        }
        Ok(())
    }
}
