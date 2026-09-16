use meowlive_protocol::{PROTOCOL_VERSION, audio::AudioFormat, control::ClientMessage};

#[test]
fn required_fields_and_unknown_message_tags_are_rejected() {
    assert!(serde_json::from_str::<ClientMessage>(r#"{"type":"hello"}"#).is_err());
    assert!(serde_json::from_str::<ClientMessage>(r#"{"type":"future"}"#).is_err());
    let hello: ClientMessage = serde_json::from_value(
        serde_json::json!({"type":"hello","protocol_version":PROTOCOL_VERSION,"future":true}),
    )
    .unwrap();
    assert!(matches!(
        hello,
        ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION
        }
    ));
}

#[test]
fn pcm_format_rejects_zero_channels_and_unbounded_sample_rates() {
    for (sample_rate, channels) in [(0, 1), (192_001, 1), (24_000, 0), (24_000, 3)] {
        assert!(
            AudioFormat {
                sample_rate,
                channels
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        AudioFormat {
            sample_rate: 24_000,
            channels: 1
        }
        .validate()
        .is_ok()
    );
}
