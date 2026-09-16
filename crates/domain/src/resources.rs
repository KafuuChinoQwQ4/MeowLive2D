//! 角色与音色资源的纯领域类型和目录级不变量。

use crate::character::CharacterProfile;
use std::{collections::HashSet, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceLanguage {
    Zh,
    En,
    Ja,
    Ko,
    Yue,
    Auto,
}

impl VoiceLanguage {
    pub fn new(value: &str) -> Result<Self, ResourceValidationError> {
        match value {
            "zh" => Ok(Self::Zh),
            "en" => Ok(Self::En),
            "ja" => Ok(Self::Ja),
            "ko" => Ok(Self::Ko),
            "yue" => Ok(Self::Yue),
            "auto" => Ok(Self::Auto),
            _ => Err(ResourceValidationError::InvalidLanguage),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Yue => "yue",
            Self::Auto => "auto",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceAsset(String);

impl ReferenceAsset {
    pub fn new(value: impl Into<String>) -> Result<Self, ResourceValidationError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ResourceValidationError::InvalidReferenceAsset);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub language: VoiceLanguage,
    pub reference_text: String,
    pub reference: ReferenceAsset,
    pub duration_ms: u32,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceAudioMetadata {
    pub duration_ms: u32,
    pub sample_rate: u32,
    pub channels: u16,
}

impl ReferenceAudioMetadata {
    pub fn new(
        duration_ms: u32,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Self, ResourceValidationError> {
        if !(3_000..=10_000).contains(&duration_ms)
            || !(8_000..=48_000).contains(&sample_rate)
            || !matches!(channels, 1 | 2)
        {
            return Err(ResourceValidationError::InvalidAudioMetadata);
        }
        Ok(Self {
            duration_ms,
            sample_rate,
            channels,
        })
    }
}

impl VoiceProfile {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        language: &str,
        reference_text: impl Into<String>,
        reference: ReferenceAsset,
        audio: ReferenceAudioMetadata,
    ) -> Result<Self, ResourceValidationError> {
        let profile = Self {
            id: id.into(),
            name: name.into(),
            language: VoiceLanguage::new(language)?,
            reference_text: reference_text.into(),
            reference,
            duration_ms: audio.duration_ms,
            sample_rate: audio.sample_rate,
            channels: audio.channels,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), ResourceValidationError> {
        if !valid_uuid(&self.id) {
            return Err(ResourceValidationError::InvalidVoiceId);
        }
        if !valid_name(&self.name) {
            return Err(ResourceValidationError::InvalidName);
        }
        ReferenceAudioMetadata::new(self.duration_ms, self.sample_rate, self.channels)?;
        let text = self.reference_text.trim();
        if text.is_empty()
            || text.chars().count() > 500
            || text
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return Err(ResourceValidationError::InvalidReferenceText);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCatalog {
    pub voices: Vec<VoiceProfile>,
    pub characters: Vec<CharacterProfile>,
    pub active_voice_id: String,
    pub active_character_id: Option<String>,
}

impl Default for ResourceCatalog {
    fn default() -> Self {
        Self {
            voices: Vec::new(),
            characters: Vec::new(),
            active_voice_id: "default".into(),
            active_character_id: None,
        }
    }
}

impl ResourceCatalog {
    pub fn validate(&self) -> Result<(), ResourceValidationError> {
        if self.voices.len() > 64 {
            return Err(ResourceValidationError::TooManyVoices);
        }
        if self.characters.len() > 64 {
            return Err(ResourceValidationError::TooManyCharacters);
        }
        let mut voice_ids = HashSet::new();
        for voice in &self.voices {
            voice.validate()?;
            if !voice_ids.insert(voice.id.as_str()) {
                return Err(ResourceValidationError::DuplicateVoiceId);
            }
        }
        let voice_exists = |id: &str| id == "default" || voice_ids.contains(id);
        if !voice_exists(&self.active_voice_id) {
            return Err(ResourceValidationError::VoiceNotFound);
        }
        let mut character_ids = HashSet::new();
        for character in &self.characters {
            character.validate()?;
            if !voice_exists(&character.voice_id) {
                return Err(ResourceValidationError::VoiceNotFound);
            }
            if !character_ids.insert(character.id.as_str()) {
                return Err(ResourceValidationError::DuplicateCharacterId);
            }
        }
        if self
            .active_character_id
            .as_deref()
            .is_some_and(|id| !character_ids.contains(id))
        {
            return Err(ResourceValidationError::CharacterNotFound);
        }
        Ok(())
    }
}

pub(crate) fn valid_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value.chars().count() <= 80 && !value.chars().any(char::is_control)
}

fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase(),
        })
        && value.as_bytes()[14] == b'4'
        && matches!(value.as_bytes()[19], b'8' | b'9' | b'a' | b'b')
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceValidationError {
    InvalidVoiceId,
    InvalidCharacterId,
    InvalidReferenceAsset,
    InvalidLanguage,
    InvalidName,
    InvalidReferenceText,
    InvalidAudioMetadata,
    InvalidIntent,
    InvalidHotkeyId,
    InvalidModelId,
    InvalidMouthParameter,
    TooManyMappings,
    TooManyVoices,
    TooManyCharacters,
    DuplicateIntent,
    DuplicateVoiceId,
    DuplicateCharacterId,
    VoiceNotFound,
    CharacterNotFound,
    MappingNotFound,
}

impl fmt::Display for ResourceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidVoiceId => "音色 ID 必须是 UUID v4",
            Self::InvalidCharacterId => "角色 ID 格式无效",
            Self::InvalidReferenceAsset => "参考音频标识格式无效",
            Self::InvalidLanguage => "音色语言仅支持 zh、en、ja、ko、yue 或 auto",
            Self::InvalidName => "名称必须包含 1 至 80 个有效字符",
            Self::InvalidReferenceText => "参考文本必须包含 1 至 500 个有效字符",
            Self::InvalidAudioMetadata => "参考音频格式、采样率或时长无效",
            Self::InvalidIntent => "能力意图必须包含 1 至 40 个有效字符",
            Self::InvalidHotkeyId => "VTS 热键 ID 必须包含 1 至 128 个有效字符",
            Self::InvalidModelId => "VTS 模型 ID 必须包含 1 至 128 个有效字符",
            Self::InvalidMouthParameter => "口型参数必须是 4 至 32 位 ASCII 字母或数字",
            Self::TooManyMappings => "单个角色最多配置 32 个能力映射",
            Self::TooManyVoices => "最多保存 64 个音色",
            Self::TooManyCharacters => "最多保存 64 个角色",
            Self::DuplicateIntent => "同一角色不能包含重复能力意图",
            Self::DuplicateVoiceId => "音色 ID 重复",
            Self::DuplicateCharacterId => "角色 ID 重复",
            Self::VoiceNotFound => "音色不存在",
            Self::CharacterNotFound => "角色不存在",
            Self::MappingNotFound => "角色能力映射不存在",
        })
    }
}

impl std::error::Error for ResourceValidationError {}
