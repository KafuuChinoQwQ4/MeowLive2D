use meowlive_domain::{
    character::{CharacterMapping, CharacterProfile},
    resources::{
        ReferenceAsset, ReferenceAudioMetadata, ResourceCatalog, ResourceValidationError,
        VoiceLanguage, VoiceProfile,
    },
};

fn voice() -> VoiceProfile {
    VoiceProfile::new(
        "123e4567-e89b-42d3-a456-426614174000",
        "猫猫",
        "zh",
        "这是一段参考文本",
        ReferenceAsset::new("123e4567-e89b-12d3-a456-426614174000").unwrap(),
        ReferenceAudioMetadata::new(3_000, 8_000, 1).unwrap(),
    )
    .unwrap()
}

#[test]
fn validates_voice_profile_fields_and_languages() {
    assert_eq!(VoiceLanguage::new("yue").unwrap().as_str(), "yue");
    for language in ["zh", "en", "ja", "ko", "yue", "auto"] {
        assert!(
            VoiceProfile::new(
                "123e4567-e89b-42d3-a456-426614174000",
                "voice",
                language,
                "prompt",
                ReferenceAsset::new("asset-1").unwrap(),
                ReferenceAudioMetadata::new(3_000, 8_000, 1).unwrap(),
            )
            .is_ok()
        );
    }
    assert_eq!(
        VoiceProfile::new(
            "123e4567-e89b-42d3-a456-426614174000",
            "voice",
            "fr",
            "prompt",
            ReferenceAsset::new("asset-1").unwrap(),
            ReferenceAudioMetadata::new(3_000, 8_000, 1).unwrap(),
        ),
        Err(ResourceValidationError::InvalidLanguage),
    );
    assert_eq!(
        VoiceProfile::new(
            "123e4567-e89b-42d3-a456-426614174000",
            "猫".repeat(81),
            "zh",
            "prompt",
            ReferenceAsset::new("asset-1").unwrap(),
            ReferenceAudioMetadata::new(3_000, 8_000, 1).unwrap(),
        ),
        Err(ResourceValidationError::InvalidName)
    );
    assert_eq!(
        VoiceProfile::new(
            "123e4567-e89b-42d3-a456-426614174000",
            "voice",
            "zh",
            "字".repeat(501),
            ReferenceAsset::new("asset-1").unwrap(),
            ReferenceAudioMetadata::new(3_000, 8_000, 1).unwrap(),
        ),
        Err(ResourceValidationError::InvalidReferenceText)
    );
    assert!(ReferenceAsset::new("../secret").is_err());
}

#[test]
fn character_mapping_validation_is_private_and_updates_reset_preview_state() {
    let mapping = CharacterMapping::new("wave", "hotkey-1", Some("fallback-1".into())).unwrap();
    assert!(!mapping.validated());
    let mut profile = CharacterProfile::new(
        "character-1",
        "Cat",
        "model-1",
        "123e4567-e89b-42d3-a456-426614174000",
        "ParamMouthOpenY",
        vec![mapping],
    )
    .unwrap();
    profile.mark_mapping_validated("wave").unwrap();
    assert!(profile.mappings[0].validated());

    let updated = CharacterProfile::new(
        profile.id,
        profile.name,
        profile.model_id,
        profile.voice_id,
        profile.mouth_parameter,
        vec![CharacterMapping::new("wave", "hotkey-2", None).unwrap()],
    )
    .unwrap();
    assert!(!updated.mappings[0].validated());
}

#[test]
fn catalog_starts_without_a_selected_voice() {
    let catalog = ResourceCatalog::default();
    assert_eq!(catalog.active_voice_id, "");
    catalog.validate().unwrap();
    assert!(catalog.active_character_id.is_none());
    assert!(catalog.voices.is_empty());
    assert_eq!(voice().id, "123e4567-e89b-42d3-a456-426614174000");
}

#[test]
fn rejects_invalid_vts_parameters_duplicate_intents_and_mapping_overflow() {
    for parameter in ["abc", "Param_Mouth", "A23456789012345678901234567890123"] {
        assert!(
            CharacterProfile::new(
                "character-1",
                "Cat",
                "model-1",
                "default",
                parameter,
                vec![],
            )
            .is_err()
        );
    }
    let duplicate = vec![
        CharacterMapping::new("wave", "one", None).unwrap(),
        CharacterMapping::new("wave", "two", None).unwrap(),
    ];
    assert_eq!(
        CharacterProfile::new(
            "character-1",
            "Cat",
            "model-1",
            "default",
            "ParamMouthOpenY",
            duplicate,
        ),
        Err(ResourceValidationError::DuplicateIntent),
    );
    let mappings = (0..33)
        .map(|index| CharacterMapping::new(format!("intent-{index}"), "hotkey", None).unwrap())
        .collect();
    assert_eq!(
        CharacterProfile::new(
            "character-1",
            "Cat",
            "model-1",
            "default",
            "ParamMouthOpenY",
            mappings,
        ),
        Err(ResourceValidationError::TooManyMappings),
    );
}
