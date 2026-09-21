use meowlive_desktop_runtime::{audio::SimulatedBackend, config::ClientConfig, playback::Playback};
use meowlive_protocol::{
    audio::{AudioChunk, AudioFormat, MAX_AUDIO_SAMPLES},
    control::ServerCommand,
    execution::ExecutionStatus,
};

#[test]
fn default_desktop_buffer_accepts_four_minutes_of_superchat_audio_in_network_chunks() {
    let config = ClientConfig::default();
    let mut player = Playback::new(
        SimulatedBackend::new(config.max_buffer_samples),
        1,
        config.max_buffer_samples,
    );
    assert!(
        player
            .command(ServerCommand::Speak {
                utterance_id: "long-sc".into(),
                generation: 1,
                format: AudioFormat {
                    sample_rate: 32_000,
                    channels: 1
                },
            })
            .unwrap()
            .is_empty()
    );
    let audio = vec![1_000; 32_000 * 240];
    let chunks = audio.len().div_ceil(MAX_AUDIO_SAMPLES);
    for (sequence, samples) in audio.chunks(MAX_AUDIO_SAMPLES).enumerate() {
        let receipts = player
            .chunk(AudioChunk {
                utterance_id: "long-sc".into(),
                generation: 1,
                sequence: sequence as u32,
                samples: samples.to_vec(),
                end: sequence + 1 == chunks,
            })
            .unwrap();
        assert!(
            receipts.is_empty(),
            "long SC was rejected at chunk {sequence}: {receipts:?}"
        );
    }
    let receipts = player.poll();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].status, ExecutionStatus::Started);
    assert!(
        player.poll().is_empty(),
        "receiving all PCM must not report playback completion"
    );
}
