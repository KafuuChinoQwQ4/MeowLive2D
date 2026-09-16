mod support;

use meowlive_adapters::speech::wav::decode_wav;

#[test]
fn decodes_pcm_and_preserves_engine_sample_rate() {
    let audio = decode_wav(&support::wav(&[0, 1000, -1000], 1)).unwrap();
    assert_eq!(audio.sample_rate, 32000);
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.samples, [0, 1000, -1000]);
}

#[test]
fn rejects_non_audio_empty_and_truncated_wav() {
    assert!(decode_wav(b"not audio").is_err());
    assert!(decode_wav(&support::wav(&[], 1)).is_err());
    let mut data = support::wav(&[100, 200], 1);
    data.pop();
    assert!(decode_wav(&data).is_err());
}

#[test]
fn rejects_unsupported_channel_layout() {
    assert!(decode_wav(&support::wav(&[1, 2, 3], 3)).is_err());
}
