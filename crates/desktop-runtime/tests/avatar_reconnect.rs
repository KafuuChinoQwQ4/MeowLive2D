mod avatar_support;
use avatar_support::*;
use std::time::Duration;

#[tokio::test]
async fn reconnect_authenticates_again_then_resets_before_accepting_new_levels() {
    let fixture = Fixture::new(true).await;
    let (levels, _, task) = fixture.start();
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    levels.send(0.9).unwrap();
    assert_eq!(injected(&mut socket).await, 0.9);
    socket.close(None).await.unwrap();
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    assert_eq!(injected(&mut socket).await, 0.0);
    levels.send(0.4).unwrap();
    assert_eq!(injected(&mut socket).await, 0.4);
    drop(levels);
    assert_eq!(injected(&mut socket).await, 0.0);
    finished(task).await;
}

#[tokio::test]
async fn unavailable_vts_does_not_block_worker_shutdown() {
    let fixture = Fixture::new(true).await;
    let config = fixture.config.clone();
    drop(fixture);
    let (levels, receiver) = tokio::sync::watch::channel(0.0);
    let (status, _) =
        tokio::sync::watch::channel(meowlive_desktop_runtime::avatar::AvatarStatus::Disabled);
    let task = tokio::spawn(meowlive_desktop_runtime::avatar::run(
        config, receiver, status,
    ));
    tokio::time::sleep(Duration::from_millis(20)).await;
    drop(levels);
    tokio::time::timeout(Duration::from_millis(200), task)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn shutdown_resets_even_when_previous_injection_has_no_response() {
    let fixture = Fixture::new(true).await;
    let (levels, _, task) = fixture.start();
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    levels.send(0.8).unwrap();
    let req = request(&mut socket, "InjectParameterDataRequest").await;
    assert_eq!(req["data"]["parameterValues"][0]["value"], 0.8);
    drop(levels);
    assert_eq!(injected(&mut socket).await, 0.0);
    finished(task).await;
}
