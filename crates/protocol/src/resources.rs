//! 角色与音色公开档案及桌面资源控制；不在跨端契约中暴露机器文件路径。
use crate::obs::{ObsOperation, ObsSettingsRequest, ObsSettingsSnapshot, ObsSnapshot};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub language: String,
    pub reference_text: String,
    pub duration_ms: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub available: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct CharacterMapping {
    pub intent: String,
    pub hotkey_id: String,
    pub fallback_hotkey_id: Option<String>,
    pub validated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct CharacterProfile {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub voice_id: String,
    pub mouth_parameter: String,
    pub mappings: Vec<CharacterMapping>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct ResourceSnapshot {
    pub voices: Vec<VoiceProfile>,
    pub characters: Vec<CharacterProfile>,
    pub active_voice_id: String,
    pub active_character_id: Option<String>,
    pub default_voice_available: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct VoiceCreateRequest {
    pub name: String,
    pub language: String,
    pub reference_text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceSelection {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct CharacterSaveRequest {
    pub id: Option<String>,
    pub name: String,
    pub model_id: String,
    pub voice_id: String,
    pub mouth_parameter: String,
    pub mappings: Vec<CharacterMapping>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct CharacterPreviewRequest {
    pub character_id: String,
    pub intent: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct VtsModel {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct VtsHotkey {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct ImportedModel {
    pub id: String,
    pub name: String,
    pub model_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DesktopResourceOperation {
    ObsSettings,
    SaveObsSettings {
        settings: ObsSettingsRequest,
    },
    Obs {
        operation: ObsOperation,
    },
    ListModels,
    LoadModel {
        model_id: String,
        mouth_parameter: String,
    },
    ListHotkeys {
        model_id: String,
    },
    TriggerHotkey {
        model_id: String,
        hotkey_id: String,
        fallback_hotkey_id: Option<String>,
    },
    /// Selection is local to the desktop. Never accept a remotely supplied path.
    ImportModel,
    ListImportedModels,
    DeleteImportedModel {
        id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DesktopResourceResult {
    ObsSettings {
        settings: ObsSettingsSnapshot,
    },
    Obs {
        snapshot: ObsSnapshot,
    },
    Models {
        models: Vec<VtsModel>,
    },
    ModelLoaded {
        model_id: String,
    },
    Hotkeys {
        model_id: String,
        hotkeys: Vec<VtsHotkey>,
    },
    HotkeyTriggered {
        hotkey_id: String,
    },
    ModelImported {
        model_name: String,
        model_file: String,
        files: u32,
        bytes: u32,
        restart_required: bool,
    },
    ImportedModels {
        models: Vec<ImportedModel>,
    },
    ModelDeleted {
        id: String,
        restart_required: bool,
    },
    Error {
        code: String,
        message: String,
    },
}
