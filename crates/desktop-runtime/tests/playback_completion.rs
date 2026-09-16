mod support;
use meowlive_desktop_runtime::{audio::BackendEvent, playback::Playback};
use meowlive_protocol::execution::ExecutionStatus;
use support::*;

#[test]
fn only_device_completion_produces_completed_receipt() {
    let device = ManualDevice::default();
    let mut player = Playback::new(device.clone(), 7, 64);
    assert!(player.command(speak(7)).unwrap().is_empty());
    assert!(player.chunk(chunk(7, 0, true)).unwrap().is_empty());
    assert!(player.poll().is_empty());
    assert!(device.0.borrow().ended);
    device.0.borrow_mut().events.push(BackendEvent::Started);
    assert_eq!(player.poll()[0].status, ExecutionStatus::Started);
    device.0.borrow_mut().events.push(BackendEvent::Completed);
    assert_eq!(player.poll()[0].status, ExecutionStatus::Completed);
    assert!(player.poll().is_empty());
}

#[test]
fn device_failure_is_visible_and_does_not_complete() {
    let device = ManualDevice::default();
    let mut player = Playback::new(device.clone(), 1, 64);
    player.command(speak(1)).unwrap();
    device
        .0
        .borrow_mut()
        .events
        .push(BackendEvent::Failed("device lost".into()));
    let receipt = player.poll().remove(0);
    assert_eq!(receipt.status, ExecutionStatus::Failed);
    assert_eq!(receipt.error.as_deref(), Some("device lost"));
}

#[test]
fn empty_utterance_fails_instead_of_waiting_forever_for_a_device_callback() {
    let mut player = Playback::new(ManualDevice::default(), 1, 64);
    player.command(speak(1)).unwrap();
    let mut empty = chunk(1, 0, true);
    empty.samples.clear();
    let receipts = player.chunk(empty).unwrap();
    assert_eq!(receipts[0].status, ExecutionStatus::Failed);
}
