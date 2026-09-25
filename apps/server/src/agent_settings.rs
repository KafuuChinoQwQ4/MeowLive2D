//! 面板 Agent 设置的本机覆盖文件；只保存偏好，不恢复运行状态。
use meowlive_application::agent::AgentSettings;
use meowlive_protocol::agent::AgentSettings as SettingsDto;
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
    settings: SettingsDto,
}

#[derive(Default)]
pub struct AgentSettingsStore {
    // Injected application states may be in-memory; bootstrap always supplies a path.
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
        .join(format!("{name}-agent.json"))
}

pub fn load_override(config_path: &Path) -> Result<Option<AgentSettings>, String> {
    let path = settings_path(config_path);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("无法读取本机 Agent 配置".into()),
    };
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err("本机 Agent 配置文件无效".into());
    }
    let bytes = std::fs::read(&path).map_err(|_| "无法读取本机 Agent 配置")?;
    let saved: SavedSettings =
        serde_json::from_slice(&bytes).map_err(|_| "本机 Agent 配置格式无效")?;
    if saved.schema != 1 {
        return Err("本机 Agent 配置版本无效".into());
    }
    let settings = AgentSettings {
        persona: saved.settings.persona,
        system_prompt: saved.settings.system_prompt,
        topic: saved.settings.topic,
        proactive_enabled: saved.settings.proactive_enabled,
        cooldown_ms: u64::from(saved.settings.cooldown_ms),
        interaction: crate::agent::mapping::interaction(saved.settings.interaction),
    };
    settings.validate().map_err(|_| "本机 Agent 配置内容无效")?;
    Ok(Some(settings))
}

impl AgentSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    // Called while holding the Agent state lock: file replacement and in-memory
    // application are ordered together, with no cancellation point between them.
    pub(crate) fn save(&self, settings: &AgentSettings) -> Result<(), String> {
        settings.validate()?;
        let Some(path) = &self.path else {
            return Ok(());
        };
        let parent = path.parent().ok_or("本机 Agent 配置目录无效")?;
        std::fs::create_dir_all(parent).map_err(|_| "无法建立本机 Agent 配置目录")?;
        if !std::fs::symlink_metadata(parent).is_ok_and(|m| m.is_dir()) {
            return Err("本机 Agent 配置目录无效".into());
        }
        let saved = SavedSettings {
            schema: 1,
            settings: SettingsDto {
                persona: settings.persona.clone(),
                system_prompt: settings.system_prompt.clone(),
                topic: settings.topic.clone(),
                proactive_enabled: settings.proactive_enabled,
                cooldown_ms: settings.cooldown_ms as u32,
                interaction: crate::agent::mapping::interaction_dto(settings.interaction.clone()),
            },
        };
        let bytes = serde_json::to_vec_pretty(&saved).map_err(|_| "无法保存 Agent 配置")?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("Agent 配置总长度超过上限".into());
        }
        let temporary = parent.join(format!(".agent-{}.tmp", uuid::Uuid::new_v4()));
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
            return Err("无法保存本机 Agent 配置，原设置保持不变".into());
        }
        Ok(())
    }
}
