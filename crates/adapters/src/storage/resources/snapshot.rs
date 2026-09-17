//! serde JSON 快照与纯领域资源目录之间的显式映射。

use meowlive_application::ports::storage::ResourceStoreError;
use meowlive_domain::{
    character::{CharacterMapping, CharacterProfile},
    resources::{ReferenceAsset, ReferenceAudioMetadata, ResourceCatalog, VoiceProfile},
};
use serde::{Deserialize, Serialize};

use super::{store_error, validate_uuid};

const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogSnapshot {
    schema_version: u32,
    voices: Vec<VoiceSnapshot>,
    characters: Vec<CharacterSnapshot>,
    active_voice_id: String,
    active_character_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoiceSnapshot {
    id: String,
    name: String,
    language: String,
    reference_text: String,
    reference: String,
    duration_ms: u32,
    sample_rate: u32,
    channels: u16,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CharacterSnapshot {
    id: String,
    name: String,
    model_id: String,
    voice_id: String,
    mouth_parameter: String,
    mappings: Vec<MappingSnapshot>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MappingSnapshot {
    intent: String,
    hotkey_id: String,
    fallback_hotkey_id: Option<String>,
    validated: bool,
}

impl CatalogSnapshot {
    pub(super) fn from_domain(catalog: &ResourceCatalog) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            voices: catalog
                .voices
                .iter()
                .map(VoiceSnapshot::from_domain)
                .collect(),
            characters: catalog
                .characters
                .iter()
                .map(CharacterSnapshot::from_domain)
                .collect(),
            active_voice_id: catalog.active_voice_id.clone(),
            active_character_id: catalog.active_character_id.clone(),
        }
    }

    pub(super) fn into_domain(self) -> Result<ResourceCatalog, ResourceStoreError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(store_error("资源目录快照版本不受支持"));
        }
        let catalog = ResourceCatalog {
            voices: self
                .voices
                .into_iter()
                .map(VoiceSnapshot::into_domain)
                .collect::<Result<_, _>>()?,
            characters: self
                .characters
                .into_iter()
                .map(CharacterSnapshot::into_domain)
                .collect::<Result<_, _>>()?,
            active_voice_id: self.active_voice_id,
            active_character_id: self.active_character_id,
        };
        if catalog
            .voices
            .iter()
            .any(|voice| voice.reference.as_str() != voice.id)
        {
            return Err(store_error("参考音频标识必须与音色标识一致"));
        }
        catalog
            .validate()
            .map_err(|_| store_error("资源目录内容无效"))?;
        Ok(catalog)
    }
}

impl VoiceSnapshot {
    fn from_domain(voice: &VoiceProfile) -> Self {
        Self {
            id: voice.id.clone(),
            name: voice.name.clone(),
            language: voice.language.as_str().into(),
            reference_text: voice.reference_text.clone(),
            reference: voice.reference.as_str().into(),
            duration_ms: voice.duration_ms,
            sample_rate: voice.sample_rate,
            channels: voice.channels,
        }
    }

    fn into_domain(self) -> Result<VoiceProfile, ResourceStoreError> {
        validate_uuid(&self.reference)?;
        let reference = ReferenceAsset::new(self.reference)
            .map_err(|_| store_error("资源目录包含无效参考音频"))?;
        let audio = ReferenceAudioMetadata::new(self.duration_ms, self.sample_rate, self.channels)
            .map_err(|_| store_error("资源目录包含无效音频信息"))?;
        VoiceProfile::new(
            self.id,
            self.name,
            &self.language,
            self.reference_text,
            reference,
            audio,
        )
        .map_err(|_| store_error("资源目录包含无效音色"))
    }
}

impl CharacterSnapshot {
    fn from_domain(character: &CharacterProfile) -> Self {
        Self {
            id: character.id.clone(),
            name: character.name.clone(),
            model_id: character.model_id.clone(),
            voice_id: character.voice_id.clone(),
            mouth_parameter: character.mouth_parameter.clone(),
            mappings: character
                .mappings
                .iter()
                .map(|mapping| MappingSnapshot {
                    intent: mapping.intent().into(),
                    hotkey_id: mapping.hotkey_id().into(),
                    fallback_hotkey_id: mapping.fallback_hotkey_id().map(str::to_owned),
                    validated: mapping.validated(),
                })
                .collect(),
        }
    }

    fn into_domain(self) -> Result<CharacterProfile, ResourceStoreError> {
        let validations: Vec<_> = self
            .mappings
            .iter()
            .map(|mapping| (mapping.intent.clone(), mapping.validated))
            .collect();
        let mappings = self
            .mappings
            .into_iter()
            .map(|mapping| {
                CharacterMapping::new(
                    mapping.intent,
                    mapping.hotkey_id,
                    mapping.fallback_hotkey_id,
                )
                .map_err(|_| store_error("资源目录包含无效角色能力映射"))
            })
            .collect::<Result<_, _>>()?;
        let mut profile = CharacterProfile::new(
            self.id,
            self.name,
            self.model_id,
            self.voice_id,
            self.mouth_parameter,
            mappings,
        )
        .map_err(|_| store_error("资源目录包含无效角色"))?;
        for (intent, validated) in validations {
            if validated {
                profile
                    .mark_mapping_validated(&intent)
                    .map_err(|_| store_error("资源目录中的能力验证状态无效"))?;
            }
        }
        Ok(profile)
    }
}
