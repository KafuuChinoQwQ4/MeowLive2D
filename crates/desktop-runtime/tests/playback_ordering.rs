mod support;
use meowlive_desktop_runtime::playback::Playback;
use support::*;

#[test]
fn independent_audio_connection_may_deliver_before_speak() {
    let device = ManualDevice::default();
    let mut player = Playback::new(device.clone(), 1, 64);
    assert!(player.chunk(chunk(1, 0, true)).unwrap().is_empty());
    player.command(speak(1)).unwrap();
    assert_eq!(device.0.borrow().samples, vec![1, 2, 3]);
    assert!(device.0.borrow().ended);
}

#[test]
fn out_of_order_duplicate_and_post_end_chunks_are_rejected() {
    for (first, second) in [(0, 0), (0, 2)] {
        let mut player = Playback::new(ManualDevice::default(), 1, 64);
        player.command(speak(1)).unwrap();
        player.chunk(chunk(1, first, false)).unwrap();
        assert!(player.chunk(chunk(1, second, true)).is_err());
    }
    let mut player = Playback::new(ManualDevice::default(), 1, 64);
    player.command(speak(1)).unwrap();
    player.chunk(chunk(1, 0, true)).unwrap();
    assert!(player.chunk(chunk(1, 1, true)).is_err());
}

#[test]
fn audio_before_control_is_bounded() {
    let mut player = Playback::new(ManualDevice::default(), 1, 4);
    player.chunk(chunk(1, 0, false)).unwrap();
    assert!(player.chunk(chunk(1, 1, true)).is_err());
}
