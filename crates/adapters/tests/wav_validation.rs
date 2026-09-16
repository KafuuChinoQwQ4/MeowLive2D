mod support;

use meowlive_adapters::speech::wav::decode_wav;

#[test]
fn rejects_stereo_block_alignment_that_disagrees_with_pcm16_frame_size() {
    let mut wav = support::wav(&[1, 2], 2);
    wav[32..34].copy_from_slice(&5u16.to_le_bytes());
    wav[28..32].copy_from_slice(&160000u32.to_le_bytes());
    assert!(decode_wav(&wav).is_err());
}

#[test]
fn rejects_sample_rates_outside_the_transport_range() {
    for sample_rate in [0u32, 7999, 96001] {
        let mut wav = support::wav(&[1, 2], 1);
        wav[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        wav[28..32].copy_from_slice(&(sample_rate * 2).to_le_bytes());
        assert!(decode_wav(&wav).is_err());
    }
}

#[test]
fn rejects_an_incomplete_stereo_frame() {
    let mut wav = support::wav(&[1, 2], 2);
    wav.truncate(wav.len() - 2);
    let riff_size = wav.len() as u32 - 8;
    wav[4..8].copy_from_slice(&riff_size.to_le_bytes());
    wav[40..44].copy_from_slice(&2u32.to_le_bytes());
    assert!(decode_wav(&wav).is_err());
}

#[test]
fn rejects_riff_lengths_that_hide_truncation_or_trailing_bytes() {
    for delta in [-1i32, 1] {
        let mut wav = support::wav(&[1, 2], 1);
        let riff_size = (wav.len() as i32 - 8 + delta) as u32;
        wav[4..8].copy_from_slice(&riff_size.to_le_bytes());
        assert!(decode_wav(&wav).is_err());
    }
    let mut wav = support::wav(&[1, 2], 1);
    wav.extend_from_slice(b"JUNK\x04\0\0\0xx");
    let riff_size = wav.len() as u32 - 8;
    wav[4..8].copy_from_slice(&riff_size.to_le_bytes());
    assert!(decode_wav(&wav).is_err());
}

#[test]
fn rejects_float_and_24_bit_audio_instead_of_silently_converting() {
    for (sample_format, bits_per_sample) in [
        (hound::SampleFormat::Float, 32),
        (hound::SampleFormat::Int, 24),
    ] {
        let mut output = std::io::Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(
            &mut output,
            hound::WavSpec {
                channels: 1,
                sample_rate: 32000,
                bits_per_sample,
                sample_format,
            },
        )
        .unwrap();
        if sample_format == hound::SampleFormat::Float {
            writer.write_sample(0.5f32).unwrap();
        } else {
            writer.write_sample(1000i32).unwrap();
        }
        writer.finalize().unwrap();
        assert!(decode_wav(&output.into_inner()).is_err());
    }
}
