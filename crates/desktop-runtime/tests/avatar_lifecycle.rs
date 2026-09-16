mod avatar_support;
use avatar_support::*;
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, avatar::AvatarStatus, lip_sync::LipSyncConfig,
    presentation::AvatarDriver,
};

#[tokio::test]
async fn disabled_avatar_requires_no_service_and_shuts_down() {
    let driver = AvatarDriver::start(Default::default()).unwrap();
    assert_eq!(*driver.status().borrow(), AvatarStatus::Disabled);
    let backend = driver
        .observe(SimulatedBackend::new(48000), LipSyncConfig::default())
        .unwrap();
    drop(backend);
    driver.shutdown().await;
}

#[tokio::test]
async fn driver_shutdown_does_not_wait_for_user_authorization() {
    let fixture = Fixture::new(false).await;
    let driver = AvatarDriver::start(fixture.config.clone()).unwrap();
    let mut socket = fixture.accept().await;
    request(&mut socket, "AuthenticationTokenRequest").await;
    tokio::time::timeout(std::time::Duration::from_millis(300), driver.shutdown())
        .await
        .unwrap();
}
