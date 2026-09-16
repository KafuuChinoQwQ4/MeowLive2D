use meowlive_desktop_runtime::audio::{BackendEvent, timing::DeviceClock};
use std::time::{Duration, Instant};

#[test]
fn device_callback_submission_does_not_mean_audio_was_played() {
    let now = Instant::now();
    let mut clock = DeviceClock::default();
    clock.submitted(now, Duration::from_millis(20), 480, 48_000);
    assert!(clock.poll(now, true, true).is_empty());
    assert_eq!(
        clock.poll(now + Duration::from_millis(20), true, true),
        vec![BackendEvent::Started]
    );
    assert!(
        clock
            .poll(now + Duration::from_millis(29), true, true)
            .is_empty()
    );
    assert_eq!(
        clock.poll(now + Duration::from_millis(30), true, true),
        vec![BackendEvent::Completed]
    );
    assert!(
        clock
            .poll(now + Duration::from_secs(1), true, true)
            .is_empty()
    );
}

#[test]
fn input_end_and_empty_device_queue_are_both_required() {
    let now = Instant::now();
    let mut clock = DeviceClock::default();
    clock.submitted(now, Duration::ZERO, 480, 48_000);
    assert_eq!(clock.poll(now, false, false), vec![BackendEvent::Started]);
    assert!(
        clock
            .poll(now + Duration::from_secs(1), false, true)
            .is_empty()
    );
    assert!(
        clock
            .poll(now + Duration::from_secs(1), true, false)
            .is_empty()
    );
    assert_eq!(
        clock.poll(now + Duration::from_secs(1), true, true),
        vec![BackendEvent::Completed]
    );
}
