mod avatar_support;
use avatar_support::*;
use meowlive_desktop_runtime::avatar::AvatarStatus;
use serde_json::json;

#[tokio::test]
async fn first_authorization_saves_private_token_only_after_authentication() {
    let fixture = Fixture::new(false).await;
    let (levels, _, task) = fixture.start();
    let mut socket = fixture.accept().await;
    let req = request(&mut socket, "AuthenticationTokenRequest").await;
    assert_eq!(req["data"]["pluginName"], "MeowLive2D");
    assert!(!fixture.config.token_path.exists());
    respond(
        &mut socket,
        &req,
        "AuthenticationTokenResponse",
        json!({"authenticationToken":"cached-private-token"}),
    )
    .await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    let token: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.config.token_path).unwrap()).unwrap();
    assert_eq!(token["authenticationToken"], "cached-private-token");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&fixture.config.token_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    drop(levels);
    assert_eq!(injected(&mut socket).await, 0.0);
    finished(task).await;
}

#[tokio::test]
async fn denied_authorization_stops_without_another_popup_or_token_file() {
    let fixture = Fixture::new(false).await;
    let (_levels, status, task) = fixture.start();
    let mut socket = fixture.accept().await;
    let req = request(&mut socket, "AuthenticationTokenRequest").await;
    respond(
        &mut socket,
        &req,
        "APIError",
        json!({"errorID":50,"message":"private-server-detail"}),
    )
    .await;
    finished(task).await;
    assert_eq!(*status.borrow(), AvatarStatus::AuthorizationRequired);
    assert!(!fixture.config.token_path.exists());
}

#[tokio::test]
async fn revoked_cached_token_stops_without_requesting_new_authorization() {
    let fixture = Fixture::new(true).await;
    let (_levels, status, task) = fixture.start();
    let mut socket = fixture.accept().await;
    let req = request(&mut socket, "AuthenticationRequest").await;
    respond(
        &mut socket,
        &req,
        "AuthenticationResponse",
        json!({"authenticated":false,"reason":"cached-private-token"}),
    )
    .await;
    finished(task).await;
    assert_eq!(*status.borrow(), AvatarStatus::AuthorizationRequired);
}

#[tokio::test]
async fn authorization_wait_is_cancelled_when_audio_owner_exits() {
    let fixture = Fixture::new(false).await;
    let (levels, _, task) = fixture.start();
    let mut socket = fixture.accept().await;
    request(&mut socket, "AuthenticationTokenRequest").await;
    drop(levels);
    tokio::time::timeout(std::time::Duration::from_millis(200), task)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn token_is_reused_after_first_authentication_connection_is_lost() {
    let fixture = Fixture::new(false).await;
    let (levels, _, task) = fixture.start();
    let mut socket = fixture.accept().await;
    let req = request(&mut socket, "AuthenticationTokenRequest").await;
    respond(
        &mut socket,
        &req,
        "AuthenticationTokenResponse",
        json!({"authenticationToken":"cached-private-token"}),
    )
    .await;
    request(&mut socket, "AuthenticationRequest").await;
    assert!(!fixture.config.token_path.exists());
    socket.close(None).await.unwrap();
    let mut socket = fixture.accept().await;
    authenticate(&mut socket).await;
    assert_eq!(injected(&mut socket).await, 0.0);
    assert!(fixture.config.token_path.exists());
    drop(levels);
    assert_eq!(injected(&mut socket).await, 0.0);
    finished(task).await;
}
