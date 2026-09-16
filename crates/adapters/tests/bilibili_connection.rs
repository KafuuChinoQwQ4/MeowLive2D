mod bilibili_support;

use bilibili_support::{FixtureMode, PlatformFixture};
use meowlive_adapters::live::bilibili::{BilibiliConfig, BilibiliLiveSource};
use meowlive_application::ports::live_source::LiveSource;
use meowlive_domain::event::EventKind;
use std::time::Duration;

fn config(fixture: &PlatformFixture) -> BilibiliConfig {
    BilibiliConfig {
        app_id: 42,
        access_key_id: "test-key".into(),
        access_key_secret: "test-secret".into(),
        identity_code: "identity".into(),
        api_base_url: fixture.base_url.clone(),
        request_timeout: Duration::from_secs(2),
        connect_timeout: Duration::from_secs(2),
        receive_timeout: Duration::from_secs(1),
        app_heartbeat_interval: Duration::from_secs(20),
        websocket_heartbeat_interval: Duration::from_secs(30),
        ..BilibiliConfig::default()
    }
}

#[tokio::test]
async fn connects_with_signed_start_authenticates_and_closes_the_project() {
    let fixture = PlatformFixture::start(FixtureMode::Events).await;
    let source = BilibiliLiveSource::new(config(&fixture)).unwrap();
    let mut connection = source.connect().await.unwrap();

    assert_eq!(connection.room_id(), "99");
    let chat = connection.next().await.unwrap().unwrap();
    let gift = connection.next().await.unwrap().unwrap();
    assert_eq!(chat.id, "bilibili:99:dm-1");
    assert_eq!(
        chat.kind,
        EventKind::Chat {
            text: "hello".into()
        }
    );
    assert_eq!(gift.id, "bilibili:99:gift-1");
    assert_eq!(
        gift.kind,
        EventKind::Gift {
            name: "Cat".into(),
            count: 2
        }
    );

    connection.close().await.unwrap();
    connection.close().await.unwrap();
    let observed = fixture.observed().await;
    assert_eq!(
        observed.start_bodies,
        [r#"{"code":"identity","app_id":42}"#]
    );
    assert_eq!(observed.end_bodies, [r#"{"app_id":42,"game_id":"game-1"}"#]);
    assert_eq!(observed.auth_bodies, ["auth-body"]);
    assert!(observed.start_signature_valid);
}

#[tokio::test]
async fn authentication_rejection_ends_the_created_project() {
    let fixture = PlatformFixture::start(FixtureMode::RejectAuth).await;
    let source = BilibiliLiveSource::new(config(&fixture)).unwrap();

    let error = match source.connect().await {
        Ok(_) => panic!("authentication rejection must fail"),
        Err(error) => error,
    };
    assert!(!error.retryable);
    assert!(!error.message.contains("auth-body"));
    assert!(!error.message.contains("test-secret"));
    assert_eq!(fixture.observed().await.end_bodies.len(), 1);
}

#[tokio::test]
async fn accepts_an_empty_authority_reply() {
    let fixture = PlatformFixture::start(FixtureMode::EmptyAuth).await;
    let source = BilibiliLiveSource::new(config(&fixture)).unwrap();
    let mut connection = source.connect().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::test]
async fn interaction_end_terminates_the_connection_without_an_event() {
    let fixture = PlatformFixture::start(FixtureMode::InteractionEnd).await;
    let source = BilibiliLiveSource::new(config(&fixture)).unwrap();
    let mut connection = source.connect().await.unwrap();
    let error = connection.next().await.unwrap_err();
    assert!(!error.retryable);
    assert_eq!(error.message, "哔哩哔哩互动场次已结束");
    connection.close().await.unwrap();
}

#[tokio::test]
async fn incomplete_start_data_ends_the_project_when_game_id_is_known() {
    let fixture = PlatformFixture::start(FixtureMode::IncompleteStart).await;
    let source = BilibiliLiveSource::new(config(&fixture)).unwrap();
    assert!(source.connect().await.is_err());
    assert_eq!(fixture.observed().await.end_bodies.len(), 1);
}

#[tokio::test]
async fn malformed_successful_start_response_is_non_retryable_unknown() {
    let fixture = PlatformFixture::start(FixtureMode::MalformedStart).await;
    let source = BilibiliLiveSource::new(config(&fixture)).unwrap();
    let error = match source.connect().await {
        Ok(_) => panic!("malformed start must fail"),
        Err(error) => error,
    };
    assert!(!error.retryable);
    assert_eq!(error.message, "哔哩哔哩启动请求结果未知");
    assert!(fixture.observed().await.end_bodies.is_empty());
}

#[tokio::test]
async fn tries_the_next_websocket_url_when_the_first_cannot_connect() {
    let fixture = PlatformFixture::start(FixtureMode::AlternateWebsocket).await;
    let source = BilibiliLiveSource::new(config(&fixture)).unwrap();
    let mut connection = source.connect().await.unwrap();
    connection.close().await.unwrap();
    assert_eq!(fixture.observed().await.auth_bodies, ["auth-body"]);
}

#[tokio::test]
async fn start_timeout_is_non_retryable_because_remote_creation_is_unknown() {
    let fixture = PlatformFixture::start(FixtureMode::SlowStart).await;
    let mut settings = config(&fixture);
    settings.request_timeout = Duration::from_secs(1);
    let source = BilibiliLiveSource::new(settings).unwrap();
    let error = match source.connect().await {
        Ok(_) => panic!("timed out start must fail"),
        Err(error) => error,
    };
    assert!(!error.retryable);
    assert_eq!(error.message, "哔哩哔哩启动请求结果未知");
    assert!(fixture.observed().await.end_bodies.is_empty());
}

#[tokio::test]
async fn silent_websocket_times_out_and_close_still_ends_the_project() {
    let fixture = PlatformFixture::start(FixtureMode::Silent).await;
    let mut settings = config(&fixture);
    settings.receive_timeout = Duration::from_secs(2);
    settings.app_heartbeat_interval = Duration::from_secs(1);
    let source = BilibiliLiveSource::new(settings).unwrap();
    let mut connection = source.connect().await.unwrap();

    let error = connection.next().await.unwrap_err();
    assert!(error.retryable);
    assert_eq!(error.message, "哔哩哔哩长连接接收超时");
    connection.close().await.unwrap();
    let observed = fixture.observed().await;
    assert_eq!(observed.heartbeat_bodies, [r#"{"game_id":"game-1"}"#]);
    assert_eq!(observed.end_bodies.len(), 1);
}

#[tokio::test]
async fn cancelling_next_keeps_close_usable() {
    let fixture = PlatformFixture::start(FixtureMode::Silent).await;
    let source = BilibiliLiveSource::new(config(&fixture)).unwrap();
    let mut connection = source.connect().await.unwrap();

    assert!(
        tokio::time::timeout(Duration::from_millis(10), connection.next())
            .await
            .is_err()
    );
    connection.close().await.unwrap();
    assert_eq!(fixture.observed().await.end_bodies.len(), 1);
}

#[test]
fn production_endpoint_requires_https_but_loopback_http_is_allowed() {
    let mut invalid = BilibiliConfig {
        app_id: 1,
        access_key_id: "key".into(),
        access_key_secret: "secret".into(),
        identity_code: "code".into(),
        api_base_url: "http://example.com".into(),
        ..BilibiliConfig::default()
    };
    assert!(BilibiliLiveSource::new(invalid.clone()).is_err());
    invalid.api_base_url = "http://127.0.0.1:1234".into();
    assert!(BilibiliLiveSource::new(invalid).is_ok());

    let oversized = BilibiliConfig {
        app_id: 1,
        access_key_id: "x".repeat(257),
        access_key_secret: "secret".into(),
        identity_code: "code".into(),
        ..BilibiliConfig::default()
    };
    assert!(BilibiliLiveSource::new(oversized).is_err());
}
