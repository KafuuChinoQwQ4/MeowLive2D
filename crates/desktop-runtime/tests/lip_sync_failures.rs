mod lip_sync_support;
use lip_sync_support::{MeteredDevice, format};
use meowlive_desktop_runtime::{
    audio::AudioBackend,
    lip_sync::{LipSyncBackend, LipSyncConfig},
};
use std::time::Duration;
use tokio::sync::watch;

#[tokio::test(start_paused = true)]
async fn each_device_operation_failure_stops_output_and_clears_the_mouth() {
    for operation in ["start", "push", "finish"] {
        let device = MeteredDevice::default();
        let (tx, rx) = watch::channel(0.0);
        let mut backend =
            LipSyncBackend::new(device.clone(), LipSyncConfig::default(), tx).unwrap();
        backend.start(format()).unwrap();
        device.0.borrow_mut().level = 0.5;
        tokio::time::advance(Duration::from_millis(40)).await;
        backend.poll();
        assert!(*rx.borrow() > 0.0);
        device.0.borrow_mut().fail_operation = Some(operation);
        let result = match operation {
            "start" => backend.start(format()),
            "push" => backend.push(&[16000; 10]),
            "finish" => backend.finish(),
            _ => unreachable!(),
        };
        assert_eq!(result.unwrap_err(), format!("device {operation} failed"));
        assert_eq!(*rx.borrow(), 0.0);
        assert!(device.0.borrow().stopped);
        tokio::time::advance(Duration::from_millis(40)).await;
        backend.poll();
        assert_eq!(*rx.borrow(), 0.0, "failed output must not reopen");
    }
}
