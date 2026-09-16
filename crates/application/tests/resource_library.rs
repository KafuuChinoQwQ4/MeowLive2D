use std::sync::{Arc, Mutex};

use meowlive_application::{
    ports::storage::{ResourceStore, ResourceStoreError, StoredReference},
    resources::{ResolvedVoice, ResourceError, ResourceLibrary},
};
use meowlive_domain::{
    character::{CharacterMapping, CharacterProfile},
    resources::{ReferenceAsset, ReferenceAudioMetadata, ResourceCatalog, VoiceProfile},
};

#[derive(Default)]
struct TestStore {
    catalog: Mutex<ResourceCatalog>,
    fail_save: Mutex<bool>,
    removed: Mutex<Vec<String>>,
    stored: Mutex<usize>,
}

impl ResourceStore for TestStore {
    fn load(&self) -> Result<ResourceCatalog, ResourceStoreError> {
        Ok(self.catalog.lock().unwrap().clone())
    }
    fn save(&self, catalog: &ResourceCatalog) -> Result<(), ResourceStoreError> {
        if *self.fail_save.lock().unwrap() {
            return Err(ResourceStoreError::new("save failed"));
        }
        *self.catalog.lock().unwrap() = catalog.clone();
        Ok(())
    }
    fn new_voice_id(&self) -> Result<String, ResourceStoreError> {
        Ok("123e4567-e89b-42d3-a456-426614174000".into())
    }
    fn store_reference(
        &self,
        voice_id: &str,
        _wav: &[u8],
    ) -> Result<StoredReference, ResourceStoreError> {
        *self.stored.lock().unwrap() += 1;
        Ok(StoredReference {
            reference: ReferenceAsset::new(voice_id).unwrap(),
            audio: ReferenceAudioMetadata::new(3_000, 8_000, 1).unwrap(),
        })
    }
    fn remove_reference(&self, reference: &ReferenceAsset) -> Result<(), ResourceStoreError> {
        self.removed.lock().unwrap().push(reference.as_str().into());
        Ok(())
    }
    fn resolve_reference(&self, reference: &ReferenceAsset) -> Result<String, ResourceStoreError> {
        Ok(format!("/engine/references/{}.wav", reference.as_str()))
    }
}

fn character(hotkey: &str) -> CharacterProfile {
    CharacterProfile::new(
        "character-1",
        "Cat",
        "model-1",
        "default",
        "ParamMouthOpenY",
        vec![CharacterMapping::new("wave", hotkey, None).unwrap()],
    )
    .unwrap()
}

#[test]
fn failed_save_does_not_publish_in_memory_changes() {
    let store = Arc::new(TestStore::default());
    let library = ResourceLibrary::open(store.clone()).unwrap();
    *store.fail_save.lock().unwrap() = true;
    let before = library.snapshot();
    assert!(library.create_character(character("hotkey-1")).is_err());
    assert_eq!(library.snapshot(), before);
}

#[test]
fn failed_voice_catalog_publish_removes_the_unpublished_audio() {
    let store = Arc::new(TestStore::default());
    let library = ResourceLibrary::open(store.clone()).unwrap();
    *store.fail_save.lock().unwrap() = true;
    assert!(
        library
            .create_voice("voice", "zh", "prompt", b"wav")
            .is_err()
    );
    assert!(library.snapshot().voices.is_empty());
    assert_eq!(*store.stored.lock().unwrap(), 1);
    assert_eq!(store.removed.lock().unwrap().len(), 1);
}

#[test]
fn full_voice_catalog_rejects_before_writing_audio() {
    let store = Arc::new(TestStore::default());
    {
        let mut catalog = store.catalog.lock().unwrap();
        for index in 0..64_u64 {
            let id = format!("00000000-0000-4000-8000-{index:012x}");
            catalog.voices.push(
                VoiceProfile::new(
                    &id,
                    format!("voice-{index}"),
                    "zh",
                    "prompt",
                    ReferenceAsset::new(id.clone()).unwrap(),
                    ReferenceAudioMetadata::new(3_000, 8_000, 1).unwrap(),
                )
                .unwrap(),
            );
        }
    }
    let library = ResourceLibrary::open(store.clone()).unwrap();
    assert_eq!(
        library.create_voice("overflow", "zh", "prompt", b"wav"),
        Err(ResourceError::Capacity),
    );
    assert_eq!(*store.stored.lock().unwrap(), 0);
}

#[test]
fn character_selection_selects_its_voice_and_preview_mark_is_compare_and_set() {
    let store = Arc::new(TestStore::default());
    let library = ResourceLibrary::open(store).unwrap();
    library.create_character(character("hotkey-1")).unwrap();
    let expected = library.snapshot().characters[0].clone();
    library
        .mark_mapping_validated_if_current(&expected, "wave")
        .unwrap();
    assert!(library.snapshot().characters[0].mappings[0].validated());

    library.update_character(character("hotkey-2")).unwrap();
    assert_eq!(
        library.mark_mapping_validated_if_current(&expected, "wave"),
        Err(ResourceError::Conflict),
    );
    library.select_character("character-1").unwrap();
    assert_eq!(library.snapshot().active_voice_id, "default");
}

#[test]
fn default_voice_resolves_without_a_reference() {
    let library = ResourceLibrary::memory();
    assert_eq!(
        library.resolve_voice("default").unwrap(),
        ResolvedVoice::Default
    );
}
