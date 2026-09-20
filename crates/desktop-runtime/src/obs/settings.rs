//! Windows 执行端 OBS 设置持久化；私有文件原子替换，公开快照不携带密码。
use super::ObsConfig;
use meowlive_protocol::obs::{ObsSettingsRequest, ObsSettingsSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

static SETTINGS_LOCK: Mutex<()> = Mutex::new(());
static NEXT_FILE: AtomicU64 = AtomicU64::new(1);
const MAX_FILE: u64 = 32 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredSettings {
    enabled: bool,
    websocket_url: String,
    password: Option<String>,
}

fn snapshot(config: &ObsConfig, password: Option<&str>) -> ObsSettingsSnapshot {
    ObsSettingsSnapshot {
        enabled: config.enabled,
        websocket_url: config.websocket_url.clone(),
        password_configured: password.is_some(),
        storage_available: config.settings_path.is_some(),
    }
}

fn check_path(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("OBS 设置文件路径不能包含符号链接".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("无法检查 OBS 设置文件".into()),
        }
    }
    Ok(())
}

fn valid_password(password: &str) -> bool {
    !password.is_empty() && password.len() <= 4096 && !password.chars().any(char::is_control)
}

fn load(mut config: ObsConfig) -> Result<(ObsConfig, Option<String>), String> {
    config.validate()?;
    if let Some(path) = &config.settings_path {
        check_path(path)?;
        match fs::File::open(path) {
            Ok(file) => {
                let metadata = file.metadata().map_err(|_| "无法读取 OBS 设置文件")?;
                if !metadata.is_file() || metadata.len() > MAX_FILE {
                    return Err("OBS 设置文件格式无效".into());
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if metadata.permissions().mode() & 0o077 != 0 {
                        return Err("OBS 设置文件须仅允许当前用户访问".into());
                    }
                }
                let mut bytes = Vec::new();
                file.take(MAX_FILE + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "无法读取 OBS 设置文件")?;
                if bytes.len() as u64 > MAX_FILE {
                    return Err("OBS 设置文件超过大小限制".into());
                }
                let saved: StoredSettings =
                    serde_json::from_slice(&bytes).map_err(|_| "OBS 设置文件格式无效")?;
                if saved
                    .password
                    .as_deref()
                    .is_some_and(|value| !valid_password(value))
                {
                    return Err("OBS 设置文件密码格式无效".into());
                }
                config.enabled = saved.enabled;
                config.websocket_url = saved.websocket_url;
                config.validate()?;
                // A saved empty password deliberately suppresses the original environment fallback.
                return Ok((config, saved.password));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("无法读取 OBS 设置文件".into()),
        }
    }
    let password = std::env::var(&config.password_env)
        .ok()
        .filter(|value| !value.is_empty());
    Ok((config, password))
}

pub(super) async fn resolve(config: &ObsConfig) -> Result<(ObsConfig, Option<String>), String> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || load(config))
        .await
        .map_err(|_| "OBS 设置读取任务失败")?
}

pub async fn settings(config: &ObsConfig) -> Result<ObsSettingsSnapshot, String> {
    let (config, password) = resolve(config).await?;
    Ok(snapshot(&config, password.as_deref()))
}

pub async fn save_settings(
    config: &ObsConfig,
    request: ObsSettingsRequest,
) -> Result<ObsSettingsSnapshot, String> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || {
        let _guard = SETTINGS_LOCK.lock().map_err(|_| "OBS 设置保存不可用")?;
        let (mut config, current_password) = load(config)?;
        if request
            .password
            .as_deref()
            .is_some_and(|value| !valid_password(value))
            || (request.clear_password && request.password.is_some())
        {
            return Err("请输入有效密码，或单独选择清除已保存密码".into());
        }
        if request.websocket_url != config.websocket_url
            && request.password.is_none()
            && !request.clear_password
        {
            return Err("更改 OBS 地址后，请重新输入密码或明确清除密码".into());
        }
        config.enabled = request.enabled;
        config.websocket_url = request.websocket_url;
        config.validate()?;
        let path = config
            .settings_path
            .as_ref()
            .ok_or("当前执行端没有配置文件位置，无法保存 OBS 设置")?;
        let password = if request.clear_password {
            None
        } else {
            request.password.or(current_password)
        };
        let bytes = serde_json::to_vec(&StoredSettings {
            enabled: config.enabled,
            websocket_url: config.websocket_url.clone(),
            password: password.clone(),
        })
        .map_err(|_| "无法编码 OBS 设置")?;
        write_atomic(path, &bytes)?;
        Ok(snapshot(&config, password.as_deref()))
    })
    .await
    .map_err(|_| "OBS 设置保存任务失败")?
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    check_path(path)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|_| "无法创建 OBS 设置目录")?;
    }
    check_path(path)?;
    let mut name = path
        .file_name()
        .ok_or("OBS 设置文件位置无效")?
        .to_os_string();
    name.push(format!(
        ".tmp-{}-{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let temporary = path.with_file_name(name);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|_| "无法创建 OBS 私有设置文件")?;
    let result = (|| {
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| "无法保存 OBS 设置文件")?;
        // Close before replacement on Windows; std::fs::rename uses MoveFileExW with REPLACE_EXISTING.
        drop(file);
        check_path(path)?;
        fs::rename(&temporary, path).map_err(|_| "无法替换 OBS 设置文件".to_owned())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
