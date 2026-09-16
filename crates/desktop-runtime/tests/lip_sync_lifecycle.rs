mod lip_sync_support;
use lip_sync_support::{MeteredDevice, format};
use meowlive_desktop_runtime::{
    audio::{AudioBackend, BackendEvent},
    lip_sync::{LipSyncBackend, LipSyncConfig},
};
use std::time::Duration;
use tokio::sync::watch;

#[tokio::test(start_paused = true)]
async fn received_pcm_does_not_open_mouth_without_device_output() {
    let device = MeteredDevice::default();
    let (tx, rx) = watch::channel(0.0);
    let mut backend = LipSyncBackend::new(device, LipSyncConfig::default(), tx).unwrap();
    backend.start(format()).unwrap();
    backend.push(&[32000; 1024]).unwrap();
    tokio::time::advance(Duration::from_millis(40)).await;
    backend.poll();
    assert_eq!(*rx.borrow(), 0.0);
}

#[tokio::test(start_paused = true)]
async fn observed_playback_opens_mouth_but_stop_and_drop_reset_immediately() {
    let device = MeteredDevice::default();
    let (tx, rx) = watch::channel(0.0);
    let mut backend = LipSyncBackend::new(device.clone(), LipSyncConfig::default(), tx).unwrap();
    backend.start(format()).unwrap();
    device.0.borrow_mut().level = 0.2;
    tokio::time::advance(Duration::from_millis(40)).await;
    backend.poll();
    assert!(*rx.borrow() > 0.0);
    backend.stop();
    assert_eq!(*rx.borrow(), 0.0);
    assert!(device.0.borrow().stopped);
    backend.start(format()).unwrap();
    tokio::time::advance(Duration::from_millis(40)).await;
    backend.poll();
    assert!(*rx.borrow() > 0.0);
    drop(backend);
    assert_eq!(*rx.borrow(), 0.0);
    assert!(device.0.borrow().stopped);
}

#[tokio::test(start_paused = true)]
async fn device_completed_or_failed_resets_without_changing_receipts() {
    for terminal in [
        BackendEvent::Completed,
        BackendEvent::Failed("device lost".into()),
    ] {
        let device = MeteredDevice::default();
        let (tx, rx) = watch::channel(0.0);
        let mut backend =
            LipSyncBackend::new(device.clone(), LipSyncConfig::default(), tx).unwrap();
        backend.start(format()).unwrap();
        device.0.borrow_mut().level = 0.4;
        tokio::time::advance(Duration::from_millis(40)).await;
        backend.poll();
        assert!(*rx.borrow() > 0.0);
        device.0.borrow_mut().events.push(terminal.clone());
        assert_eq!(backend.poll(), vec![terminal]);
        assert_eq!(*rx.borrow(), 0.0);
    }
}
