use meowlive_desktop_runtime::audio::{AudioBackend, BackendEvent, SimulatedBackend};
use meowlive_protocol::audio::AudioFormat;
use std::time::Duration;

fn simulator() -> SimulatedBackend {
    let mut backend = SimulatedBackend::new(80_000);
    backend
        .start(AudioFormat {
            sample_rate: 8_000,
            channels: 1,
        })
        .unwrap();
    backend
}

#[test]
fn receiving_future_loud_pcm_does_not_replace_current_silence() {
    let mut backend = simulator();
    backend.push(&vec![0; 800]).unwrap();
    backend.push(&vec![16_384; 8_000]).unwrap();
    assert_eq!(backend.output_level(), 0.0);
    std::thread::sleep(Duration::from_millis(130));
    assert_eq!(backend.output_level(), 0.5);
}

#[test]
fn silence_inside_one_network_chunk_retains_its_playback_position() {
    let mut backend = simulator();
    let mut samples = vec![0; 800];
    samples.extend(vec![-16_384; 8_000]);
    backend.push(&samples).unwrap();
    assert_eq!(backend.output_level(), 0.0);
    std::thread::sleep(Duration::from_millis(130));
    assert_eq!(backend.output_level(), 0.5);
}

#[test]
fn stopped_and_finished_simulation_cannot_retain_loud_energy() {
    let mut backend = simulator();
    backend.push(&vec![16_384; 8_000]).unwrap();
    assert_eq!(backend.output_level(), 0.5);
    backend.stop();
    assert_eq!(backend.output_level(), 0.0);
    backend
        .start(AudioFormat {
            sample_rate: 8_000,
            channels: 1,
        })
        .unwrap();
    assert_eq!(backend.output_level(), 0.0);
    backend.push(&vec![16_384; 160]).unwrap();
    backend.finish().unwrap();
    std::thread::sleep(Duration::from_millis(40));
    assert_eq!(backend.output_level(), 0.0);
    assert_eq!(
        backend.poll(),
        vec![BackendEvent::Started, BackendEvent::Completed]
    );
    assert_eq!(backend.output_level(), 0.0);
}

#[test]
fn underrun_expires_energy_before_a_new_chunk_starts() {
    let mut backend = simulator();
    backend.push(&vec![16_384; 160]).unwrap();
    std::thread::sleep(Duration::from_millis(40));
    assert_eq!(backend.output_level(), 0.0);
    backend.push(&vec![-8_192; 8_000]).unwrap();
    assert_eq!(backend.output_level(), 0.25);
}

#[test]
fn rejected_buffer_overflow_does_not_replace_accepted_energy() {
    let mut backend = simulator();
    backend.push(&vec![0; 8_000]).unwrap();
    assert!(backend.push(&vec![16_384; 80_000]).is_err());
    assert_eq!(backend.output_level(), 0.0);
}
