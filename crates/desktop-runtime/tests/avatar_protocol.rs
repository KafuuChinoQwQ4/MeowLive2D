mod avatar_support;
use avatar_support::*;
use futures_util::SinkExt;
use meowlive_desktop_runtime::avatar::AvatarStatus;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn wrong_response_type_fails_without_exposing_response_data() {
    let fixture = Fixture::new(true).await;
    let (_levels, status, task) = fixture.start();
    let mut socket = fixture.accept().await;
    let req = request(&mut socket, "AuthenticationRequest").await;
    respond(
        &mut socket,
        &req,
        "ParameterCreationResponse",
        json!({"authenticated":true,"secret":"cached-private-token"}),
    )
    .await;
    finished(task).await;
    let result = format!("{:?}", status.borrow());
    assert!(matches!(*status.borrow(), AvatarStatus::Failed(_)));
    assert!(!result.contains("cached-private-token"));
}

#[tokio::test]
async fn wrong_request_id_is_never_accepted_and_total_deadline_survives_pings() {
    let fixture = Fixture::new(true).await;
    let (levels, mut status, task) = fixture.start();
    let mut socket = fixture.accept().await;
    let mut req = request(&mut socket, "AuthenticationRequest").await;
    req["requestID"] = json!("unrelated");
    respond(
        &mut socket,
        &req,
        "AuthenticationResponse",
        json!({"authenticated":true}),
    )
    .await;
    let spam = tokio::spawn(async move {
        loop {
            if socket.send(Message::Ping(vec![1].into())).await.is_err() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });
    status_is(&mut status, AvatarStatus::Disconnected).await;
    drop(levels);
    finished(task).await;
    spam.abort();
}

#[tokio::test]
async fn api_parameter_error_is_visible_and_sanitized() {
    let fixture = Fixture::new(true).await;
    let (_levels, status, task) = fixture.start();
    let mut socket = fixture.accept().await;
    let req = request(&mut socket, "AuthenticationRequest").await;
    respond(
        &mut socket,
        &req,
        "AuthenticationResponse",
        json!({"authenticated":true}),
    )
    .await;
    let req = request(&mut socket, "ParameterCreationRequest").await;
    respond(
        &mut socket,
        &req,
        "APIError",
        json!({"errorID":450,"message":"private-file-path"}),
    )
    .await;
    finished(task).await;
    assert!(matches!(*status.borrow(), AvatarStatus::Failed(_)));
    assert!(!format!("{:?}", status.borrow()).contains("private-file-path"));
}
