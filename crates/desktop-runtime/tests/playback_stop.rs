mod support;
use meowlive_desktop_runtime::{audio::BackendEvent, playback::Playback};
use meowlive_protocol::{control::ServerCommand, execution::ExecutionStatus};
use support::*;

#[test]
fn stop_cancels_and_drops_old_generation_audio_and_device_events() {
    let device = ManualDevice::default();
    let mut player = Playback::new(device.clone(), 1, 64);
    player.command(speak(1)).unwrap();
    player.chunk(chunk(1, 0, false)).unwrap();
    device.0.borrow_mut().events.push(BackendEvent::Completed);
    let receipts = player
        .command(ServerCommand::Stop { generation: 2 })
        .unwrap();
    assert_eq!(receipts[0].status, ExecutionStatus::Cancelled);
    assert!(device.0.borrow().stopped);
    assert!(player.chunk(chunk(1, 1, true)).unwrap().is_empty());
    assert!(player.poll().is_empty());
}

#[test]
fn disconnect_stops_audio_and_does_not_claim_completion() {
    let device = ManualDevice::default();
    let mut player = Playback::new(device.clone(), 1, 64);
    player.command(speak(1)).unwrap();
    player.disconnect();
    assert!(device.0.borrow().stopped);
    assert!(player.poll().is_empty());
}

#[test]
fn next_generation_audio_can_race_a_stop_on_the_control_socket() {
    let device = ManualDevice::default();
    let mut player = Playback::new(device.clone(), 1, 64);
    player.command(speak(1)).unwrap();
    player.chunk(chunk(2, 0, true)).unwrap();
    player
        .command(ServerCommand::Stop { generation: 2 })
        .unwrap();
    player.command(speak(2)).unwrap();
    assert_eq!(device.0.borrow().samples, vec![1, 2, 3]);
    assert!(device.0.borrow().ended);
}
