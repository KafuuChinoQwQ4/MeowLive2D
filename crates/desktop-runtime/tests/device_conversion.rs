use meowlive_desktop_runtime::audio::conversion::PcmConverter;
use meowlive_protocol::audio::AudioFormat;

#[test]
fn mono_pcm_is_duplicated_and_resampled_to_default_stereo_device() {
    let mut converter = PcmConverter::new(
        AudioFormat {
            sample_rate: 24_000,
            channels: 1,
        },
        48_000,
        2,
    )
    .unwrap();
    assert_eq!(
        converter.convert(&[16384, -16384]).unwrap(),
        vec![0.5, 0.5, 0.5, 0.5, -0.5, -0.5, -0.5, -0.5]
    );
}

#[test]
fn resampling_phase_is_preserved_across_network_chunks() {
    let format = AudioFormat {
        sample_rate: 44_100,
        channels: 1,
    };
    let mut whole = PcmConverter::new(format, 48_000, 2).unwrap();
    let mut split = PcmConverter::new(format, 48_000, 2).unwrap();
    let source: Vec<_> = (0..1000).collect();
    let expected = whole.convert(&source).unwrap();
    let mut actual = split.convert(&source[..337]).unwrap();
    actual.extend(split.convert(&source[337..]).unwrap());
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 2176);
}

#[test]
fn converter_rejects_partial_interleaved_frames() {
    let mut converter = PcmConverter::new(
        AudioFormat {
            sample_rate: 24_000,
            channels: 2,
        },
        48_000,
        2,
    )
    .unwrap();
    assert!(converter.convert(&[1, 2, 3]).is_err());
}
