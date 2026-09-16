mod avatar_support;
use avatar_support::*;
use std::time::Duration;

#[tokio::test]
async fn cached_authentication_creates_parameter_and_streams_latest_finite_levels() {
    let fixture = Fixture::new(true).await;
    let (levels, _, task) = fixture.start();
    levels.send(0.8).unwrap();
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    levels.send(0.2).unwrap();
    levels.send(0.7).unwrap();
    assert_eq!(injected(&mut socket).await, 0.7);
    levels.send(9.0).unwrap();
    assert_eq!(injected(&mut socket).await, 1.0);
    levels.send(f64::NAN).unwrap();
    assert_eq!(injected(&mut socket).await, 0.0);
    drop(levels);
    assert_eq!(injected(&mut socket).await, 0.0);
    finished(task).await;
}

#[tokio::test]
async fn unchanged_levels_heartbeat_and_expire_without_producer_freshness() {
    let fixture = Fixture::new(true).await;
    let (levels, _, task) = fixture.start();
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    levels.send(0.6).unwrap();
    assert_eq!(injected(&mut socket).await, 0.6);
    let start = tokio::time::Instant::now();
    let mut count = 0;
    while start.elapsed() < Duration::from_millis(300) {
        injected(&mut socket).await;
        count += 1;
    }
    assert!(count <= 11, "must not exceed 30 Hz");
    assert_eq!(injected(&mut socket).await, 0.0);
    drop(levels);
    assert_eq!(injected(&mut socket).await, 0.0);
    finished(task).await;
}

#[tokio::test]
async fn stale_level_resets_while_previous_injection_has_no_response() {
    let mut fixture = Fixture::new(true).await;
    fixture.config.request_timeout_ms = 1000;
    let (levels, _, task) = fixture.start();
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    levels.send(0.9).unwrap();
    let req = request(&mut socket, "InjectParameterDataRequest").await;
    assert_eq!(req["data"]["parameterValues"][0]["value"], 0.9);
    let reset = tokio::time::timeout(Duration::from_millis(350), injected(&mut socket)).await;
    assert_eq!(
        reset.expect("stale samples must close the mouth before the network deadline"),
        0.0
    );
    drop(levels);
    assert_eq!(injected(&mut socket).await, 0.0);
    finished(task).await;
}
