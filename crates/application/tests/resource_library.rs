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
    fail_remove: Mutex<bool>,
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
        if *self.fail_remove.lock().unwrap() {
            return Err(ResourceStoreError::new("cleanup failed"));
        }
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

#[test]
fn deleting_voice_clears_selection_and_character_binding_and_persists() {
    let store = Arc::new(TestStore::default());
    let library = ResourceLibrary::open(store.clone()).unwrap();
    let voice = library
        .create_voice("voice", "zh", "prompt", b"wav")
        .unwrap();
    let mut role = character("hotkey-1");
    role.voice_id = voice.id.clone();
    library.create_character(role).unwrap();
    library.select_character("character-1").unwrap();
    library.delete_voice(&voice.id).unwrap();
    let snapshot = library.snapshot();
    assert!(snapshot.voices.is_empty());
    assert_eq!(snapshot.active_voice_id, "");
    assert!(snapshot.active_character_id.is_none());
    assert_eq!(snapshot.characters[0].voice_id, "");
    assert_eq!(
        *store.removed.lock().unwrap(),
        vec![voice.reference.as_str()]
    );
    assert_eq!(ResourceLibrary::open(store).unwrap().snapshot(), snapshot);
}

#[test]
fn failed_voice_delete_save_keeps_voice_file_and_selection() {
    let store = Arc::new(TestStore::default());
    let library = ResourceLibrary::open(store.clone()).unwrap();
    let voice = library
        .create_voice("voice", "zh", "prompt", b"wav")
        .unwrap();
    library.select_voice(&voice.id).unwrap();
    let before = library.snapshot();
    *store.fail_save.lock().unwrap() = true;
    assert!(library.delete_voice(&voice.id).is_err());
    assert_eq!(library.snapshot(), before);
    assert!(store.removed.lock().unwrap().is_empty());
}

#[test]
fn deleting_selected_character_keeps_voice_but_clears_character_selection() {
    let library = ResourceLibrary::memory();
    library.create_character(character("hotkey-1")).unwrap();
    library.select_character("character-1").unwrap();
    library.delete_character("character-1").unwrap();
    assert!(library.snapshot().characters.is_empty());
    assert!(library.snapshot().active_character_id.is_none());
    assert_eq!(library.snapshot().active_voice_id, "default");
    assert_eq!(
        library.delete_character("missing"),
        Err(ResourceError::CharacterNotFound)
    );
    assert!(library.delete_voice("default").is_err());
}

#[test]
fn reference_cleanup_can_retry_after_catalog_deletion_and_restart() {
    let store = Arc::new(TestStore::default());
    let library = ResourceLibrary::open(store.clone()).unwrap();
    let voice = library
        .create_voice("voice", "zh", "prompt", b"wav")
        .unwrap();
    *store.fail_remove.lock().unwrap() = true;
    assert!(
        library
            .delete_voice(&voice.id)
            .unwrap_err()
            .to_string()
            .contains("清理失败")
    );
    assert!(library.snapshot().voices.is_empty());
    *store.fail_remove.lock().unwrap() = false;
    let reopened = ResourceLibrary::open(store.clone()).unwrap();
    reopened.delete_voice(&voice.id).unwrap();
    assert_eq!(
        *store.removed.lock().unwrap(),
        vec![voice.reference.as_str()]
    );
    assert!(reopened.delete_voice("../outside").is_err());
}
