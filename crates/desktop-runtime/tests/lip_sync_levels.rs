use meowlive_desktop_runtime::lip_sync::{LipSync, LipSyncConfig};
use std::time::Duration;

#[test]
fn noise_gate_gain_and_smoothing_follow_elapsed_playback_time() {
    let mut mouth = LipSync::new(LipSyncConfig::default()).unwrap();
    assert_eq!(mouth.sample(0.005, Duration::from_millis(33)), 0.0);
    let opened = mouth.sample(0.1, Duration::from_millis(33));
    assert!(opened > 0.1 && opened < 1.0, "{opened}");
    let released = mouth.sample(0.0, Duration::from_millis(33));
    assert!(released < opened && released > 0.0);
    assert_eq!(mouth.reset(), 0.0);
    assert_eq!(mouth.sample(0.0, Duration::from_millis(33)), 0.0);
}

#[test]
fn smoothing_is_independent_of_poll_frequency() {
    let mut once = LipSync::new(LipSyncConfig::default()).unwrap();
    let mut repeated = LipSync::new(LipSyncConfig::default()).unwrap();
    let expected = once.sample(0.1, Duration::from_millis(100));
    let mut actual = 0.0;
    for _ in 0..10 {
        actual = repeated.sample(0.1, Duration::from_millis(10));
    }
    assert!((expected - actual).abs() < 1e-8);
}

#[test]
fn invalid_levels_close_mouth_and_large_levels_are_clamped() {
    let mut mouth = LipSync::new(LipSyncConfig::default()).unwrap();
    assert!(mouth.sample(100.0, Duration::from_secs(1)) <= 1.0);
    assert_eq!(mouth.sample(f32::NAN, Duration::from_secs(1)), 0.0);
    assert_eq!(mouth.sample(f32::INFINITY, Duration::from_secs(1)), 0.0);
    assert_eq!(mouth.sample(-1.0, Duration::from_secs(1)), 0.0);
}

#[test]
fn config_rejects_nonfinite_and_unbounded_values() {
    for config in [
        LipSyncConfig {
            gain: f64::NAN,
            ..Default::default()
        },
        LipSyncConfig {
            noise_floor: 1.0,
            ..Default::default()
        },
        LipSyncConfig {
            attack_ms: 0,
            ..Default::default()
        },
        LipSyncConfig {
            release_ms: 6000,
            ..Default::default()
        },
        LipSyncConfig {
            update_hz: 0,
            ..Default::default()
        },
        LipSyncConfig {
            update_hz: 60,
            ..Default::default()
        },
    ] {
        assert!(config.validate().is_err());
    }
}
