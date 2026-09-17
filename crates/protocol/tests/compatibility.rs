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

#[test]
fn training_modes_are_explicit_and_old_requests_keep_text_review() {
    use meowlive_protocol::training::TrainingCreateRequest;
    let mut request = serde_json::json!({
        "name": "声音训练", "voice_id": "voice-1", "sovits_epochs": 1,
        "gpt_epochs": 1, "reviewed": true, "clips": [{"text": "你好", "language": "zh"}]
    });
    let legacy: TrainingCreateRequest = serde_json::from_value(request.clone()).unwrap();
    assert_eq!(legacy.performance.batch_size, 1);
    assert_eq!(legacy.performance.data_workers, 1);
    assert_eq!(legacy.performance.cpu_threads, 2);
    assert_eq!(legacy.performance.gpu_index, 0);
    assert!(legacy.performance.low_memory);
    assert_eq!(
        serde_json::to_value(legacy).unwrap()["text_mode"],
        "reviewed_text"
    );
    request["text_mode"] = serde_json::json!("audio_only");
    request["reviewed"] = serde_json::json!(false);
    request["clips"] = serde_json::json!([{"language": "zh"}]);
    let audio: TrainingCreateRequest = serde_json::from_value(request.clone()).unwrap();
    let serialized = serde_json::to_value(audio).unwrap();
    assert_eq!(serialized["clips"][0]["text"], "");
    assert_eq!(serialized["text_mode"], "audio_only");
    request["text_mode"] = serde_json::json!("unknown");
    assert!(serde_json::from_value::<TrainingCreateRequest>(request).is_err());
}

#[test]
fn training_performance_round_trips_on_create_requests_and_jobs() {
    use meowlive_protocol::training::{TrainingCreateRequest, TrainingJob};
    let performance = serde_json::json!({
        "batch_size": 4, "data_workers": 0, "cpu_threads": 8,
        "gpu_index": 2, "low_memory": false
    });
    let request: TrainingCreateRequest = serde_json::from_value(serde_json::json!({
        "name": "声音训练", "voice_id": "voice-1", "sovits_epochs": 1,
        "gpt_epochs": 1, "reviewed": true, "clips": [{"text": "你好", "language": "zh"}],
        "performance": performance
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap()["performance"],
        performance
    );

    let job: TrainingJob = serde_json::from_value(serde_json::json!({
        "id": "job", "name": "声音训练", "voice_id": "voice-1", "status": "queued",
        "progress": 0, "message": "已排队", "clip_count": 2,
        "created_at_ms": 1, "updated_at_ms": 1, "version_id": null
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(job).unwrap()["performance"]["cpu_threads"],
        2
    );
}
