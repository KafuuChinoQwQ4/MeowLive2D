use meowlive_domain::speech::{SpeechText, SpeechValidationError};
use meowlive_domain::voice::VoiceId;

#[test]
fn rejects_blank_speech() {
    for text in ["", " \t\n", "\u{3000}"] {
        assert_eq!(SpeechText::new(text), Err(SpeechValidationError::EmptyText));
    }
}

#[test]
fn validates_the_limit_by_unicode_characters() {
    assert!(SpeechText::new("喵".repeat(500)).is_ok());
    assert_eq!(
        SpeechText::new("喵".repeat(501)),
        Err(SpeechValidationError::TextTooLong { max: 500 })
    );
}

#[test]
fn combined_broadcast_has_a_separate_bounded_unicode_limit() {
    assert!(SpeechText::broadcast("猫".repeat(1200)).is_ok());
    assert_eq!(
        SpeechText::broadcast("猫".repeat(1201)),
        Err(SpeechValidationError::TextTooLong { max: 1200 })
    );
    assert_eq!(
        SpeechText::broadcast(" \t\n"),
        Err(SpeechValidationError::EmptyText)
    );
    assert_eq!(
        SpeechText::broadcast("原文\0回复"),
        Err(SpeechValidationError::InvalidText)
    );
    assert!(SpeechText::new("猫".repeat(501)).is_err());
}

#[test]
fn trims_surrounding_whitespace_without_changing_spoken_content() {
    let text = SpeechText::new(" \n你好， 世界！\t").unwrap();
    assert_eq!(text.as_str(), "你好， 世界！");
}

#[test]
fn rejects_control_characters_but_accepts_speech_whitespace() {
    for text in ["你好\0世界", "你好\u{7f}世界"] {
        assert_eq!(
            SpeechText::new(text),
            Err(SpeechValidationError::InvalidText)
        );
    }
    assert!(SpeechText::new("第一行\n第二行\t结束").is_ok());
}

#[test]
fn accepts_portable_voice_identifiers() {
    for id in ["default", "cat_1", "voice-zh", "A".repeat(64).as_str()] {
        assert_eq!(VoiceId::new(id).unwrap().as_str(), id);
    }
}

#[test]
fn rejects_empty_whitespace_path_and_oversized_voice_identifiers() {
    for id in ["", " ", "default voice", "../voice", "voice/name", "猫"] {
        assert_eq!(VoiceId::new(id), Err(SpeechValidationError::InvalidVoiceId));
    }
    assert_eq!(
        VoiceId::new("v".repeat(65)),
        Err(SpeechValidationError::InvalidVoiceId)
    );
}
