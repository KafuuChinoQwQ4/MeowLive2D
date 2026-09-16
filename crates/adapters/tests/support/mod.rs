use std::io::Cursor;

pub fn wav(samples: &[i16], channels: u16) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels,
        sample_rate: 32000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(&mut output, spec).unwrap();
    for sample in samples {
        writer.write_sample(*sample).unwrap();
    }
    writer.finalize().unwrap();
    output.into_inner()
}
