//! 角色和音色档案管理、参考素材登记与能力校验；文件读写通过存储接口完成。

use crate::ports::storage::{ResourceStore, ResourceStoreError, StoredReference};
use meowlive_domain::{
    character::CharacterProfile,
    resources::{
        ReferenceAudioMetadata, ResourceCatalog, ResourceValidationError, VoiceLanguage,
        VoiceProfile,
    },
};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedVoice {
    Default,
    Uploaded {
        id: String,
        reference_audio: String,
        reference_text: String,
        language: VoiceLanguage,
    },
}

impl ResolvedVoice {
    pub fn reference_audio(&self) -> Option<&str> {
        match self {
            Self::Default => None,
            Self::Uploaded {
                reference_audio, ..
            } => Some(reference_audio),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceError {
    Invalid(ResourceValidationError),
    Store(String),
    VoiceNotFound,
    CharacterNotFound,
    Conflict,
    Capacity,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(formatter, "{error}"),
            Self::Store(message) => formatter.write_str(message),
            Self::VoiceNotFound => formatter.write_str("音色不存在"),
            Self::CharacterNotFound => formatter.write_str("角色不存在"),
            Self::Conflict => formatter.write_str("预览期间资源已修改，请重新预览"),
            Self::Capacity => formatter.write_str("资源数量已达到上限"),
        }
    }
}

impl std::error::Error for ResourceError {}

impl From<ResourceValidationError> for ResourceError {
    fn from(value: ResourceValidationError) -> Self {
        Self::Invalid(value)
    }
}

impl From<ResourceStoreError> for ResourceError {
    fn from(value: ResourceStoreError) -> Self {
        Self::Store(value.message)
    }
}

pub struct ResourceLibrary {
    store: Arc<dyn ResourceStore>,
    catalog: RwLock<ResourceCatalog>,
}

impl ResourceLibrary {
    pub fn open(store: Arc<dyn ResourceStore>) -> Result<Self, ResourceError> {
        let catalog = store.load()?;
        catalog.validate()?;
        Ok(Self {
            store,
            catalog: RwLock::new(catalog),
        })
    }

    pub fn memory() -> Self {
        Self::open(Arc::new(MemoryResourceStore::default())).expect("default catalog is valid")
    }

    pub fn snapshot(&self) -> ResourceCatalog {
        self.catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn create_voice(
        &self,
        name: &str,
        language: &str,
        reference_text: &str,
        wav: &[u8],
    ) -> Result<VoiceProfile, ResourceError> {
        // Validate user-controlled text before allocating an immutable asset.
        VoiceLanguage::new(language)?;
        let placeholder = meowlive_domain::resources::ReferenceAsset::new(
            "00000000-0000-4000-8000-000000000000",
        )?;
        VoiceProfile::new(
            "00000000-0000-4000-8000-000000000000",
            name,
            language,
            reference_text,
            placeholder,
            ReferenceAudioMetadata::new(3_000, 8_000, 1)?,
        )?;
        let mut guard = self
            .catalog
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.voices.len() >= 64 {
            return Err(ResourceError::Capacity);
        }
        let voice_id = self.store.new_voice_id()?;
        if guard.voices.iter().any(|voice| voice.id == voice_id) {
            return Err(ResourceError::Conflict);
        }
        let stored = self.store.store_reference(&voice_id, wav)?;
        let profile =
            match voice_from_stored(&voice_id, name, language, reference_text, stored.clone()) {
                Ok(profile) => profile,
                Err(error) => {
                    let _ = self.store.remove_reference(&stored.reference);
                    return Err(error);
                }
            };
        let mut next = guard.clone();
        next.voices.push(profile.clone());
        if let Err(error) = self.store.save(&next) {
            if self.store.remove_reference(&stored.reference).is_err() {
                return Err(ResourceError::Store(
                    "资源目录保存失败，且未发布的参考音频清理失败".into(),
                ));
            }
            return Err(error.into());
        }
        *guard = next;
        Ok(profile)
    }

    pub fn create_character(&self, profile: CharacterProfile) -> Result<(), ResourceError> {
        profile.validate()?;
        self.update_catalog(|catalog| {
            if catalog.characters.len() >= 64 {
                return Err(ResourceError::Capacity);
            }
            if catalog.characters.iter().any(|item| item.id == profile.id) {
                return Err(ResourceError::Conflict);
            }
            if !voice_exists(catalog, &profile.voice_id) {
                return Err(ResourceError::VoiceNotFound);
            }
            catalog.characters.push(reset_validation(profile));
            Ok(())
        })
    }

    pub fn update_character(&self, profile: CharacterProfile) -> Result<(), ResourceError> {
        profile.validate()?;
        self.update_catalog(|catalog| {
            if !voice_exists(catalog, &profile.voice_id) {
                return Err(ResourceError::VoiceNotFound);
            }
            let index = catalog
                .characters
                .iter()
                .position(|item| item.id == profile.id)
                .ok_or(ResourceError::CharacterNotFound)?;
            catalog.characters[index] = reset_validation(profile);
            Ok(())
        })
    }

    pub fn delete_voice(&self, id: &str) -> Result<(), ResourceError> {
        {
            let catalog = self.catalog.read().unwrap_or_else(|p| p.into_inner());
            if !catalog.voices.iter().any(|voice| voice.id == id) {
                // FileResourceStore owns references/<voice UUID>.wav. Metadata may
                // already be committed by a previous deletion whose cleanup failed.
                if !meowlive_domain::resources::valid_voice_id(id)
                    || catalog
                        .voices
                        .iter()
                        .any(|voice| voice.reference.as_str() == id)
                {
                    return Err(ResourceError::VoiceNotFound);
                }
                let reference = meowlive_domain::resources::ReferenceAsset::new(id)?;
                return self.store.remove_reference(&reference).map_err(|error| {
                    ResourceError::Store(format!(
                        "音色配置已删除，但参考音频清理失败，请重试：{error}"
                    ))
                });
            }
        }
        let reference = self.update_catalog(|catalog| {
            let index = catalog
                .voices
                .iter()
                .position(|voice| voice.id == id)
                .ok_or(ResourceError::VoiceNotFound)?;
            let removed = catalog.voices.remove(index);
            if catalog.active_voice_id == id {
                catalog.active_voice_id.clear();
            }
            for character in &mut catalog.characters {
                if character.voice_id == id {
                    character.voice_id.clear();
                    if catalog.active_character_id.as_deref() == Some(&character.id) {
                        catalog.active_character_id = None;
                    }
                }
            }
            Ok(removed.reference)
        })?;
        // Commit metadata first, so a failed catalog write cannot destroy a live reference.
        self.store.remove_reference(&reference).map_err(|error| {
            ResourceError::Store(format!(
                "音色配置已删除，但参考音频清理失败，请重试：{error}"
            ))
        })
    }

    pub fn delete_character(&self, id: &str) -> Result<(), ResourceError> {
        self.update_catalog(|catalog| {
            let index = catalog
                .characters
                .iter()
                .position(|character| character.id == id)
                .ok_or(ResourceError::CharacterNotFound)?;
            catalog.characters.remove(index);
            if catalog.active_character_id.as_deref() == Some(id) {
                catalog.active_character_id = None;
            }
            Ok(())
        })
    }

    pub fn clear_voice_selection_if_current(&self, id: &str) -> Result<(), ResourceError> {
        self.update_catalog(|catalog| {
            if catalog.active_voice_id == id {
                catalog.active_voice_id.clear();
                catalog.active_character_id = None;
            }
            Ok(())
        })
    }

    pub fn select_voice(&self, id: &str) -> Result<(), ResourceError> {
        self.update_catalog(|catalog| {
            if !voice_exists(catalog, id) {
                return Err(ResourceError::VoiceNotFound);
            }
            catalog.active_voice_id = id.to_owned();
            Ok(())
        })
    }

    pub fn select_character(&self, id: &str) -> Result<(), ResourceError> {
        self.update_catalog(|catalog| {
            let character = catalog
                .characters
                .iter()
                .find(|item| item.id == id)
                .ok_or(ResourceError::CharacterNotFound)?;
            catalog.active_voice_id.clone_from(&character.voice_id);
            catalog.active_character_id = Some(id.to_owned());
            Ok(())
        })
    }

    pub fn mark_mapping_validated(
        &self,
        character_id: &str,
        intent: &str,
    ) -> Result<(), ResourceError> {
        self.update_catalog(|catalog| {
            let character = catalog
                .characters
                .iter_mut()
                .find(|item| item.id == character_id)
                .ok_or(ResourceError::CharacterNotFound)?;
            character.mark_mapping_validated(intent)?;
            Ok(())
        })
    }

    pub fn mark_mapping_validated_if_current(
        &self,
        expected: &CharacterProfile,
        intent: &str,
    ) -> Result<(), ResourceError> {
        self.update_catalog(|catalog| {
            let character = catalog
                .characters
                .iter_mut()
                .find(|item| item.id == expected.id)
                .ok_or(ResourceError::CharacterNotFound)?;
            if character != expected {
                return Err(ResourceError::Conflict);
            }
            character.mark_mapping_validated(intent)?;
            Ok(())
        })
    }

    pub fn resolve_voice(&self, id: &str) -> Result<ResolvedVoice, ResourceError> {
        if id == "default" {
            return Ok(ResolvedVoice::Default);
        }
        let profile = self
            .snapshot()
            .voices
            .into_iter()
            .find(|voice| voice.id == id)
            .ok_or(ResourceError::VoiceNotFound)?;
        let reference_audio = self.store.resolve_reference(&profile.reference)?;
        Ok(ResolvedVoice::Uploaded {
            id: profile.id,
            reference_audio,
            reference_text: profile.reference_text,
            language: profile.language,
        })
    }

    fn update_catalog<T>(
        &self,
        update: impl FnOnce(&mut ResourceCatalog) -> Result<T, ResourceError>,
    ) -> Result<T, ResourceError> {
        let mut guard = self
            .catalog
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut next = guard.clone();
        let result = update(&mut next)?;
        next.validate()?;
        self.store.save(&next)?;
        *guard = next;
        Ok(result)
    }
}

fn voice_from_stored(
    voice_id: &str,
    name: &str,
    language: &str,
    reference_text: &str,
    stored: StoredReference,
) -> Result<VoiceProfile, ResourceError> {
    Ok(VoiceProfile::new(
        voice_id,
        name,
        language,
        reference_text,
        stored.reference,
        stored.audio,
    )?)
}

fn voice_exists(catalog: &ResourceCatalog, id: &str) -> bool {
    id == "default" || catalog.voices.iter().any(|voice| voice.id == id)
}

fn reset_validation(profile: CharacterProfile) -> CharacterProfile {
    CharacterProfile::new(
        profile.id,
        profile.name,
        profile.model_id,
        profile.voice_id,
        profile.mouth_parameter,
        profile
            .mappings
            .into_iter()
            .map(|mapping| CharacterMappingParts::from(mapping).into_mapping())
            .collect(),
    )
    .expect("previously validated profile")
}

struct CharacterMappingParts {
    intent: String,
    hotkey: String,
    fallback: Option<String>,
}
impl From<meowlive_domain::character::CharacterMapping> for CharacterMappingParts {
    fn from(value: meowlive_domain::character::CharacterMapping) -> Self {
        Self {
            intent: value.intent().to_owned(),
            hotkey: value.hotkey_id().to_owned(),
            fallback: value.fallback_hotkey_id().map(str::to_owned),
        }
    }
}
impl CharacterMappingParts {
    fn into_mapping(self) -> meowlive_domain::character::CharacterMapping {
        meowlive_domain::character::CharacterMapping::new(self.intent, self.hotkey, self.fallback)
            .expect("previously validated mapping")
    }
}

#[derive(Default)]
struct MemoryResourceStore {
    catalog: RwLock<ResourceCatalog>,
}
impl ResourceStore for MemoryResourceStore {
    fn load(&self) -> Result<ResourceCatalog, ResourceStoreError> {
        Ok(self.catalog.read().unwrap().clone())
    }
    fn save(&self, catalog: &ResourceCatalog) -> Result<(), ResourceStoreError> {
        *self.catalog.write().unwrap() = catalog.clone();
        Ok(())
    }
    fn new_voice_id(&self) -> Result<String, ResourceStoreError> {
        Err(ResourceStoreError::new("内存资源库不能创建音色"))
    }
    fn store_reference(
        &self,
        _voice_id: &str,
        _wav: &[u8],
    ) -> Result<StoredReference, ResourceStoreError> {
        Err(ResourceStoreError::new("内存资源库不能存储音频"))
    }
    fn remove_reference(
        &self,
        _reference: &meowlive_domain::resources::ReferenceAsset,
    ) -> Result<(), ResourceStoreError> {
        Ok(())
    }
    fn resolve_reference(
        &self,
        _reference: &meowlive_domain::resources::ReferenceAsset,
    ) -> Result<String, ResourceStoreError> {
        Err(ResourceStoreError::new("内存资源库没有音频文件"))
    }
}
