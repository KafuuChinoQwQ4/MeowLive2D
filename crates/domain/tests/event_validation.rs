use meowlive_domain::event::{EventKind, LiveEvent};

fn chat() -> LiveEvent {
    LiveEvent {
        id: "chat-1".into(),
        source: "simulator".into(),
        viewer: "小猫".into(),
        occurred_at_ms: 1,
        kind: EventKind::Chat {
            text: "你好\n主播\t！".into(),
        },
    }
}

#[test]
fn accepts_unicode_chat_and_identity_limits() {
    let mut event = chat();
    event.id = "x".repeat(128);
    event.source = "s".repeat(32);
    event.viewer = "猫".repeat(64);
    event.kind = EventKind::Chat {
        text: "喵".repeat(500),
    };
    assert!(event.validate().is_ok());
    assert!(chat().validate().is_ok());
}

#[test]
fn rejects_blank_oversized_or_non_ascii_identity() {
    for id in [
        "".into(),
        " ".into(),
        "x".repeat(129),
        "事件".into(),
        "bad\n".into(),
    ] {
        let mut event = chat();
        event.id = id;
        assert!(event.validate().is_err());
    }
    for source in ["".into(), "x".repeat(33), "来源".into(), "bad\t".into()] {
        let mut event = chat();
        event.source = source;
        assert!(event.validate().is_err());
    }
    for viewer in ["".into(), "猫".repeat(65), "bad\0".into(), "bad\n".into()] {
        let mut event = chat();
        event.viewer = viewer;
        assert!(event.validate().is_err());
    }
}

#[test]
fn rejects_empty_oversized_and_control_chat() {
    for text in [" ".into(), "喵".repeat(501), "bad\0".into(), "bad\r".into()] {
        let mut event = chat();
        event.kind = EventKind::Chat { text };
        assert!(event.validate().is_err());
    }
}

#[test]
fn bounds_gift_name_and_positive_count() {
    for (name, count, valid) in [
        ("猫".repeat(100), 10000, true),
        ("花".into(), 1, true),
        ("花".into(), 0, false),
        ("花".into(), 10001, false),
        ("".into(), 1, false),
        ("猫".repeat(101), 1, false),
        ("bad\t".into(), 1, false),
    ] {
        let mut event = chat();
        event.kind = EventKind::Gift { name, count };
        assert_eq!(event.validate().is_ok(), valid);
    }
}
