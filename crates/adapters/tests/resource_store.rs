use std::{fs, io::Cursor, path::PathBuf};

use meowlive_adapters::storage::resources::{FileResourceStore, FileResourceStoreConfig};
use meowlive_application::{ports::storage::ResourceStore, resources::ResourceLibrary};
use meowlive_domain::character::{CharacterMapping, CharacterProfile};

fn temp_root(label: &str) -> PathBuf {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/resource-store-tests")
        .join(format!("meowlive-{label}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn wav(seconds: u32, sample_rate: u32, channels: u16, amplitude: i16) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
    for index in 0..seconds * sample_rate * u32::from(channels) {
        writer
            .write_sample(if index == 0 { amplitude } else { 0 })
            .unwrap();
    }
    writer.finalize().unwrap();
    bytes.into_inner()
}

#[test]
fn persists_versioned_catalog_and_reopens_with_distinct_engine_paths() {
    let root = temp_root("resource-reopen");
    let config = FileResourceStoreConfig {
        storage_root: root.clone(),
        engine_root: "/engine/shared".into(),
    };
    let library = ResourceLibrary::open(std::sync::Arc::new(
        FileResourceStore::new(config.clone()).unwrap(),
    ))
    .unwrap();
    let one = library
        .create_voice("one", "zh", "提示一", &wav(3, 8000, 1, 100))
        .unwrap();
    assert!(uuid::Uuid::parse_str(&one.id).is_ok());
    let two = library
        .create_voice("two", "en", "prompt two", &wav(3, 8000, 2, 100))
        .unwrap();
    let character = CharacterProfile::new(
        "character-1",
        "Cat",
        "model-1",
        &one.id,
        "ParamMouthOpenY",
        vec![CharacterMapping::new("wave", "hotkey-1", None).unwrap()],
    )
    .unwrap();
    library.create_character(character).unwrap();
    library
        .mark_mapping_validated("character-1", "wave")
        .unwrap();
    library.select_character("character-1").unwrap();
    drop(library);

    let reopened =
        ResourceLibrary::open(std::sync::Arc::new(FileResourceStore::new(config).unwrap()))
            .unwrap();
    assert_eq!(reopened.snapshot().voices.len(), 2);
    let snapshot = reopened.snapshot();
    assert_eq!(snapshot.characters.len(), 1);
    assert_eq!(snapshot.active_character_id.as_deref(), Some("character-1"));
    assert_eq!(snapshot.active_voice_id, one.id.as_str());
    assert!(snapshot.characters[0].mappings[0].validated());
    let first = reopened
        .resolve_voice(&one.id)
        .unwrap()
        .reference_audio()
        .unwrap()
        .to_owned();
    let second = reopened
        .resolve_voice(&two.id)
        .unwrap()
        .reference_audio()
        .unwrap()
        .to_owned();
    assert_ne!(first, second);
    assert!(first.starts_with("/engine/shared/references/"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(root.join("catalog.json")).unwrap())
            .unwrap()["schema_version"],
        1
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_bad_duration_format_silence_size_and_missing_reference() {
    let root = temp_root("resource-wav");
    let store = std::sync::Arc::new(
        FileResourceStore::new(FileResourceStoreConfig {
            storage_root: root.clone(),
            engine_root: "/engine/shared".into(),
        })
        .unwrap(),
    );
    let library = ResourceLibrary::open(store.clone()).unwrap();
    for bytes in [
        wav(2, 8000, 1, 1),
        wav(11, 8000, 1, 1),
        wav(3, 8000, 1, 0),
        wav(3, 96000, 1, 1),
    ] {
        assert!(library.create_voice("bad", "zh", "prompt", &bytes).is_err());
    }
    assert!(
        library
            .create_voice("large", "zh", "prompt", &vec![0; 2 * 1024 * 1024 + 1])
            .is_err()
    );
    let profile = library
        .create_voice("ok", "zh", "prompt", &wav(3, 8000, 1, 1))
        .unwrap();
    let resolved = library.resolve_voice(&profile.id).unwrap();
    let local = root
        .join("references")
        .join(format!("{}.wav", profile.reference.as_str()));
    fs::remove_file(local).unwrap();
    let error = library.resolve_voice(&profile.id).unwrap_err().to_string();
    assert!(error.contains(&profile.id));
    assert!(error.contains("references/"));
    assert!(!error.contains(root.to_string_lossy().as_ref()));
    assert!(
        resolved
            .reference_audio()
            .unwrap()
            .starts_with("/engine/shared/")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_path_traversal_in_engine_root_and_corrupt_snapshot() {
    let root = temp_root("resource-corrupt");
    assert!(
        FileResourceStore::new(FileResourceStoreConfig {
            storage_root: root.clone(),
            engine_root: "relative/../engine".into()
        })
        .is_err()
    );
    fs::write(root.join("catalog.json"), br#"{"schema_version":999}"#).unwrap();
    let store = FileResourceStore::new(FileResourceStoreConfig {
        storage_root: root.clone(),
        engine_root: "/engine/shared".into(),
    })
    .unwrap();
    assert!(store.load().is_err());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn rejects_reference_directory_replaced_by_a_symbolic_link() {
    use std::os::unix::fs::symlink;

    let root = temp_root("resource-symlink");
    let outside = temp_root("resource-outside");
    let store = FileResourceStore::new(FileResourceStoreConfig {
        storage_root: root.clone(),
        engine_root: "/engine/shared".into(),
    })
    .unwrap();
    fs::remove_dir(root.join("references")).unwrap();
    symlink(&outside, root.join("references")).unwrap();
    assert!(store.load().is_err());
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
    fs::remove_file(root.join("references")).unwrap();
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
