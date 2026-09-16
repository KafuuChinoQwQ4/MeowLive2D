use meowlive_desktop_runtime::audio::meter::{OutputMeter, SampleEnergy};
use std::time::{Duration, Instant};

fn energy(samples: &[f32]) -> SampleEnergy {
    let mut energy = SampleEnergy::default();
    for &sample in samples {
        energy.add(sample);
    }
    energy
}

#[test]
fn callback_energy_is_visible_only_during_its_device_playback_interval() {
    let now = Instant::now();
    let mut meter = OutputMeter::default();
    meter.record(
        now,
        Duration::from_millis(40),
        480,
        48_000,
        energy(&[0.5, -0.5]),
    );
    assert_eq!(meter.level(now), 0.0);
    assert_eq!(meter.level(now + Duration::from_millis(39)), 0.0);
    assert_eq!(meter.level(now + Duration::from_millis(40)), 0.5);
    assert_eq!(meter.level(now + Duration::from_millis(49)), 0.5);
    assert_eq!(meter.level(now + Duration::from_millis(50)), 0.0);
    assert_eq!(meter.level(now + Duration::from_secs(5)), 0.0);
}

#[test]
fn rms_uses_all_channels_without_cancelling_opposite_polarity() {
    let now = Instant::now();
    let mut meter = OutputMeter::default();
    meter.record(
        now,
        Duration::ZERO,
        2,
        8_000,
        energy(&[0.5, -0.5, 0.0, 0.0]),
    );
    assert!((meter.level(now) - 0.353_553_38).abs() < 0.000_001);
}

#[test]
fn future_audio_does_not_fill_silent_or_underrun_intervals() {
    let now = Instant::now();
    let mut meter = OutputMeter::default();
    meter.record(now, Duration::ZERO, 80, 8_000, energy(&[0.5]));
    meter.record(now, Duration::from_millis(10), 80, 8_000, energy(&[0.0]));
    meter.record(now, Duration::from_millis(40), 80, 8_000, energy(&[0.75]));
    assert_eq!(meter.level(now + Duration::from_millis(5)), 0.5);
    assert_eq!(meter.level(now + Duration::from_millis(15)), 0.0);
    assert_eq!(meter.level(now + Duration::from_millis(30)), 0.0);
    assert_eq!(meter.level(now + Duration::from_millis(45)), 0.75);
}

#[test]
fn bounded_meter_discards_old_entries_without_replaying_them() {
    let now = Instant::now();
    let mut meter = OutputMeter::default();
    for offset in 0..1_024 {
        meter.record(
            now,
            Duration::from_millis(offset * 10),
            80,
            8_000,
            energy(&[0.25]),
        );
    }
    assert_eq!(meter.level(now), 0.0);
    assert_eq!(meter.level(now + Duration::from_millis(10_235)), 0.25);
    assert_eq!(meter.level(now + Duration::from_millis(10_240)), 0.0);
}

#[test]
fn reset_forgets_both_playing_and_future_energy() {
    let now = Instant::now();
    let mut meter = OutputMeter::default();
    meter.record(now, Duration::ZERO, 80, 8_000, energy(&[0.5]));
    meter.record(now, Duration::from_secs(1), 80, 8_000, energy(&[0.5]));
    meter.clear();
    assert_eq!(meter.level(now), 0.0);
    assert_eq!(meter.level(now + Duration::from_secs(1)), 0.0);
}

#[test]
fn invalid_samples_and_empty_blocks_cannot_publish_invalid_levels() {
    let now = Instant::now();
    let mut meter = OutputMeter::default();
    meter.record(
        now,
        Duration::ZERO,
        80,
        8_000,
        energy(&[f32::NAN, f32::INFINITY]),
    );
    assert_eq!(meter.level(now), 0.0);
    meter.record(now, Duration::ZERO, 80, 8_000, energy(&[2.0, -2.0]));
    assert_eq!(meter.level(now), 1.0);
    meter.clear();
    meter.record(now, Duration::ZERO, 0, 8_000, energy(&[0.5]));
    meter.record(now, Duration::ZERO, 80, 0, energy(&[0.5]));
    assert_eq!(meter.level(now), 0.0);
}
