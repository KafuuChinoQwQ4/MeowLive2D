mod lip_sync_support;
use lip_sync_support::{MeteredDevice, format};
use meowlive_desktop_runtime::{
    audio::AudioBackend,
    lip_sync::{LipSyncBackend, LipSyncConfig},
};
use std::time::Duration;
use tokio::sync::watch;

#[tokio::test(start_paused = true)]
async fn fast_polling_obeys_configured_cadence_and_republishes_silence_for_freshness() {
    let (tx, mut rx) = watch::channel(0.0);
    let config = LipSyncConfig {
        update_hz: 20,
        ..LipSyncConfig::default()
    };
    let mut backend = LipSyncBackend::new(MeteredDevice::default(), config, tx).unwrap();
    backend.start(format()).unwrap();
    rx.borrow_and_update();
    for _ in 0..9 {
        tokio::time::advance(Duration::from_millis(5)).await;
        backend.poll();
        assert!(!rx.has_changed().unwrap());
    }
    tokio::time::advance(Duration::from_millis(5)).await;
    backend.poll();
    assert!(rx.has_changed().unwrap());
    assert_eq!(*rx.borrow_and_update(), 0.0);
    tokio::time::advance(Duration::from_millis(50)).await;
    backend.poll();
    assert!(
        rx.has_changed().unwrap(),
        "silence is a fresh device observation too"
    );
}

#[tokio::test(start_paused = true)]
async fn slow_consumers_receive_only_the_latest_value_and_stop_bypasses_cadence() {
    let device = MeteredDevice::default();
    let (tx, mut rx) = watch::channel(0.0);
    let mut backend = LipSyncBackend::new(device.clone(), LipSyncConfig::default(), tx).unwrap();
    backend.start(format()).unwrap();
    rx.borrow_and_update();
    device.0.borrow_mut().level = 0.3;
    for _ in 0..10 {
        tokio::time::advance(Duration::from_millis(40)).await;
        backend.poll();
    }
    assert!(*rx.borrow() > 0.0);
    backend.stop();
    assert_eq!(*rx.borrow_and_update(), 0.0);
    assert!(
        !rx.has_changed().unwrap(),
        "there is no backlog of old mouth values"
    );
}
