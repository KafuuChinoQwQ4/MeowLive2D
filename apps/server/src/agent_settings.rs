//! 面板 Agent 设置与人物卡配置的本机覆盖文件；只保存偏好，不恢复运行状态。
use meowlive_application::agent::AgentSettings;
use meowlive_protocol::agent::{
    AgentSettings as SettingsDto, PersonaProfileCreateRequest, PersonaProfileIdRequest,
    PersonaProfileRenameRequest, PersonaProfileSummary, PersonaProfileUpdateRequest,
    PersonaProfilesSnapshot,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io::Write,
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 512 * 1024;
const MAX_PROFILES: usize = 32;
const MAX_NAME_CHARS: usize = 64;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSettings {
    schema: u8,
    settings: SettingsDto,
    selected_profile_id: Option<String>,
    profiles: Vec<PersonaProfileSummary>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySavedSettings {
    schema: u8,
    settings: SettingsDto,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SavedFile {
    Current(SavedSettings),
    Legacy(LegacySavedSettings),
}

#[derive(Debug)]
pub(crate) enum PersonaStoreError {
    Invalid(String),
    Conflict(String),
    Storage(String),
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
    read_saved(&settings_path(config_path))?
        .map(|saved| settings_from_dto(saved.settings))
        .transpose()
}

fn read_saved(path: &Path) -> Result<Option<SavedSettings>, String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("无法读取本机 Agent 配置".into()),
    };
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err("本机 Agent 配置文件无效".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "无法读取本机 Agent 配置")?;
    let saved = match serde_json::from_slice::<SavedFile>(&bytes)
        .map_err(|_| "本机 Agent 配置格式无效")?
    {
        SavedFile::Current(saved) => {
            if saved.schema != 2 {
                return Err("本机 Agent 配置版本无效".into());
            }
            saved
        }
        SavedFile::Legacy(saved) => {
            if saved.schema != 1 {
                return Err("本机 Agent 配置版本无效".into());
            }
            SavedSettings {
                schema: 2,
                selected_profile_id: Some("legacy".into()),
                profiles: vec![PersonaProfileSummary {
                    id: "legacy".into(),
                    name: "配置1".into(),
                    persona: saved.settings.persona.clone(),
                }],
                settings: saved.settings,
            }
        }
    };
    validate_saved(&saved)?;
    Ok(Some(saved))
}

fn settings_from_dto(settings: SettingsDto) -> Result<AgentSettings, String> {
    let settings = AgentSettings {
        persona: settings.persona,
        system_prompt: settings.system_prompt,
        topic: settings.topic,
        proactive_enabled: settings.proactive_enabled,
        cooldown_ms: u64::from(settings.cooldown_ms),
        interaction: crate::agent::mapping::interaction(settings.interaction),
    };
    settings.validate().map_err(|_| "本机 Agent 配置内容无效")?;
    Ok(settings)
}

fn settings_to_dto(settings: &AgentSettings) -> SettingsDto {
    SettingsDto {
        persona: settings.persona.clone(),
        system_prompt: settings.system_prompt.clone(),
        topic: settings.topic.clone(),
        proactive_enabled: settings.proactive_enabled,
        cooldown_ms: settings.cooldown_ms as u32,
        interaction: crate::agent::mapping::interaction_dto(settings.interaction.clone()),
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || name.chars().count() > MAX_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return Err("人物卡名称须为 1–64 个字符且不含控制字符".into());
    }
    Ok(())
}

fn validate_saved(saved: &SavedSettings) -> Result<(), String> {
    let settings = settings_from_dto(saved.settings.clone())?;
    if saved.profiles.len() > MAX_PROFILES {
        return Err("本机人物卡数量超过上限".into());
    }
    let mut ids = HashSet::new();
    for profile in &saved.profiles {
        if profile.id.trim().is_empty()
            || profile.id.len() > 64
            || profile.id.chars().any(char::is_control)
            || !ids.insert(&profile.id)
        {
            return Err("本机人物卡标识无效".into());
        }
        validate_name(&profile.name)?;
        let mut candidate = settings.clone();
        candidate.persona = profile.persona.clone();
        candidate.validate().map_err(|_| "本机人物卡内容无效")?;
    }
    if let Some(id) = saved.selected_profile_id.as_deref() {
        let selected = saved
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or("本机当前人物卡标识无效")?;
        if selected.persona != saved.settings.persona {
            return Err("本机当前人物卡与 Agent 设置不一致".into());
        }
    }
    Ok(())
}

fn empty_saved(settings: &AgentSettings) -> SavedSettings {
    SavedSettings {
        schema: 2,
        settings: settings_to_dto(settings),
        selected_profile_id: None,
        profiles: Vec::new(),
    }
}

fn next_name(saved: &SavedSettings) -> String {
    (1..=MAX_PROFILES + 1)
        .map(|number| format!("配置{number}"))
        .find(|name| !saved.profiles.iter().any(|profile| profile.name == *name))
        .unwrap_or_else(|| format!("配置{}", saved.profiles.len() + 1))
}

fn snapshot(saved: Option<&SavedSettings>, storage_available: bool) -> PersonaProfilesSnapshot {
    PersonaProfilesSnapshot {
        profiles: saved
            .map(|value| value.profiles.clone())
            .unwrap_or_default(),
        selected_profile_id: saved.and_then(|value| value.selected_profile_id.clone()),
        storage_available,
    }
}

impl AgentSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn saved(&self) -> Result<Option<SavedSettings>, PersonaStoreError> {
        self.path
            .as_ref()
            .map(|path| read_saved(path).map_err(PersonaStoreError::Storage))
            .unwrap_or(Ok(None))
    }

    pub(crate) fn profiles(&self) -> Result<PersonaProfilesSnapshot, PersonaStoreError> {
        let saved = self.saved()?;
        Ok(snapshot(saved.as_ref(), self.path.is_some()))
    }

    // Called while holding the Agent state lock: file replacement and in-memory
    // application are ordered together, with no cancellation point between them.
    pub(crate) fn save(&self, settings: &AgentSettings) -> Result<(), String> {
        settings.validate()?;
        if self.path.is_none() {
            return Ok(());
        }
        let mut saved = self
            .saved()
            .map_err(|error| match error {
                PersonaStoreError::Storage(message)
                | PersonaStoreError::Invalid(message)
                | PersonaStoreError::Conflict(message) => message,
            })?
            .unwrap_or_else(|| empty_saved(settings));
        saved.settings = settings_to_dto(settings);
        if let Some(id) = &saved.selected_profile_id {
            let profile = saved
                .profiles
                .iter_mut()
                .find(|profile| &profile.id == id)
                .ok_or("本机当前人物卡不存在")?;
            profile.persona = settings.persona.clone();
        } else {
            if saved.profiles.len() >= MAX_PROFILES {
                return Err("本机人物卡数量超过上限".into());
            }
            let id = uuid::Uuid::new_v4().to_string();
            let name = next_name(&saved);
            saved.profiles.push(PersonaProfileSummary {
                id: id.clone(),
                name,
                persona: settings.persona.clone(),
            });
            saved.selected_profile_id = Some(id);
        }
        self.persist(&saved)
    }

    pub(crate) fn create(
        &self,
        current: &AgentSettings,
        request: PersonaProfileCreateRequest,
    ) -> Result<(PersonaProfilesSnapshot, AgentSettings), PersonaStoreError> {
        self.require_path()?;
        let mut saved = self.saved()?.unwrap_or_else(|| empty_saved(current));
        if saved.profiles.len() >= MAX_PROFILES {
            return Err(PersonaStoreError::Invalid("本机人物卡数量超过上限".into()));
        }
        let name = if request.name.trim().is_empty() {
            next_name(&saved)
        } else {
            request.name.trim().to_owned()
        };
        validate_name(&name).map_err(PersonaStoreError::Invalid)?;
        let mut settings = current.clone();
        settings.persona = request.persona;
        settings.validate().map_err(PersonaStoreError::Invalid)?;
        let id = uuid::Uuid::new_v4().to_string();
        saved.profiles.push(PersonaProfileSummary {
            id: id.clone(),
            name,
            persona: settings.persona.clone(),
        });
        saved.selected_profile_id = Some(id);
        saved.settings = settings_to_dto(&settings);
        self.persist(&saved).map_err(PersonaStoreError::Storage)?;
        Ok((snapshot(Some(&saved), true), settings))
    }

    pub(crate) fn select(
        &self,
        current: &AgentSettings,
        request: PersonaProfileIdRequest,
    ) -> Result<(PersonaProfilesSnapshot, AgentSettings), PersonaStoreError> {
        self.require_path()?;
        let mut saved = self.saved()?.unwrap_or_else(|| empty_saved(current));
        let profile = saved
            .profiles
            .iter()
            .find(|profile| profile.id == request.id)
            .ok_or_else(|| PersonaStoreError::Invalid("要切换的人物卡不存在".into()))?;
        let mut settings = current.clone();
        settings.persona = profile.persona.clone();
        settings.validate().map_err(PersonaStoreError::Invalid)?;
        saved.selected_profile_id = Some(request.id);
        saved.settings = settings_to_dto(&settings);
        self.persist(&saved).map_err(PersonaStoreError::Storage)?;
        Ok((snapshot(Some(&saved), true), settings))
    }

    pub(crate) fn rename(
        &self,
        current: &AgentSettings,
        request: PersonaProfileRenameRequest,
    ) -> Result<PersonaProfilesSnapshot, PersonaStoreError> {
        self.require_path()?;
        let name = request.name.trim().to_owned();
        validate_name(&name).map_err(PersonaStoreError::Invalid)?;
        let mut saved = self.saved()?.unwrap_or_else(|| empty_saved(current));
        let profile = saved
            .profiles
            .iter_mut()
            .find(|profile| profile.id == request.id)
            .ok_or_else(|| PersonaStoreError::Invalid("要重命名的人物卡不存在".into()))?;
        profile.name = name;
        self.persist(&saved).map_err(PersonaStoreError::Storage)?;
        Ok(snapshot(Some(&saved), true))
    }

    pub(crate) fn update(
        &self,
        current: &AgentSettings,
        request: PersonaProfileUpdateRequest,
    ) -> Result<(PersonaProfilesSnapshot, AgentSettings), PersonaStoreError> {
        self.require_path()?;
        let name = request.name.trim().to_owned();
        validate_name(&name).map_err(PersonaStoreError::Invalid)?;
        let mut saved = self.saved()?.unwrap_or_else(|| empty_saved(current));
        if saved.selected_profile_id.as_deref() != Some(request.id.as_str()) {
            return Err(PersonaStoreError::Conflict(
                "当前人物卡已在其他页面切换，请重新读取后再保存".into(),
            ));
        }
        let mut settings = current.clone();
        settings.persona = request.persona;
        settings.validate().map_err(PersonaStoreError::Invalid)?;
        let profile = saved
            .profiles
            .iter_mut()
            .find(|profile| profile.id == request.id)
            .ok_or_else(|| PersonaStoreError::Invalid("要更新的人物卡不存在".into()))?;
        profile.name = name;
        profile.persona = settings.persona.clone();
        saved.settings = settings_to_dto(&settings);
        self.persist(&saved).map_err(PersonaStoreError::Storage)?;
        Ok((snapshot(Some(&saved), true), settings))
    }

    pub(crate) fn delete(
        &self,
        current: &AgentSettings,
        request: PersonaProfileIdRequest,
    ) -> Result<(PersonaProfilesSnapshot, Option<AgentSettings>), PersonaStoreError> {
        self.require_path()?;
        let mut saved = self.saved()?.unwrap_or_else(|| empty_saved(current));
        let position = saved
            .profiles
            .iter()
            .position(|profile| profile.id == request.id)
            .ok_or_else(|| PersonaStoreError::Invalid("要删除的人物卡不存在".into()))?;
        let deleting_selected = saved.selected_profile_id.as_deref() == Some(&request.id);
        saved.profiles.remove(position);
        let mut apply = None;
        if deleting_selected {
            saved.selected_profile_id = saved.profiles.first().map(|profile| profile.id.clone());
            if let Some(profile) = saved.profiles.first() {
                let mut settings = current.clone();
                settings.persona = profile.persona.clone();
                settings.validate().map_err(PersonaStoreError::Invalid)?;
                saved.settings = settings_to_dto(&settings);
                apply = Some(settings);
            }
        }
        self.persist(&saved).map_err(PersonaStoreError::Storage)?;
        Ok((snapshot(Some(&saved), true), apply))
    }

    fn require_path(&self) -> Result<&Path, PersonaStoreError> {
        self.path
            .as_deref()
            .ok_or_else(|| PersonaStoreError::Storage("此主服务未配置本机配置存储".into()))
    }

    fn persist(&self, saved: &SavedSettings) -> Result<(), String> {
        validate_saved(saved)?;
        let path = self.path.as_ref().ok_or("此主服务未配置本机配置存储")?;
        let parent = path.parent().ok_or("本机 Agent 配置目录无效")?;
        std::fs::create_dir_all(parent).map_err(|_| "无法建立本机 Agent 配置目录")?;
        if !std::fs::symlink_metadata(parent).is_ok_and(|m| m.is_dir()) {
            return Err("本机 Agent 配置目录无效".into());
        }
        let bytes = serde_json::to_vec_pretty(saved).map_err(|_| "无法保存 Agent 配置")?;
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
