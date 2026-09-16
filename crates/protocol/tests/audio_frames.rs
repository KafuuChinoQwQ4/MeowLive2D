use meowlive_protocol::audio::{AudioChunk, MAX_AUDIO_SAMPLES};

fn chunk() -> AudioChunk {
    AudioChunk {
        utterance_id: "a".into(),
        generation: 7,
        sequence: 2,
        samples: vec![-32768, -1, 0, 32767],
        end: true,
    }
}

#[test]
fn frame_round_trip_preserves_signed_little_endian_pcm() {
    let bytes = chunk().encode().unwrap();
    assert_eq!(
        &bytes[bytes.len() - 8..],
        &[0, 128, 255, 255, 0, 0, 255, 127]
    );
    assert_eq!(AudioChunk::decode(&bytes).unwrap(), chunk());
}

#[test]
fn decoder_rejects_truncated_extra_and_wrong_version_frames() {
    let bytes = chunk().encode().unwrap();
    for length in 0..bytes.len() {
        assert!(AudioChunk::decode(&bytes[..length]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(AudioChunk::decode(&extra).is_err());
    let mut wrong_version = bytes;
    wrong_version[4] = 99;
    assert!(AudioChunk::decode(&wrong_version).is_err());
}

#[test]
fn encoding_limits_payload_and_identity_before_allocation() {
    let mut value = chunk();
    value.samples = vec![0; MAX_AUDIO_SAMPLES + 1];
    assert!(value.encode().is_err());
    value.samples.clear();
    value.utterance_id = "x".repeat(129);
    assert!(value.encode().is_err());
}
