mod support;
use axum::{body::Body, http::Request};
use meowlive_server::{config::AppConfig, transport::http::router};
use tower::ServiceExt;

#[tokio::test]
async fn live_status_is_available_but_platform_connection_is_disabled_by_default() {
    let response = router(support::state())
        .oneshot(
            Request::builder()
                .uri("/api/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let (_, status) = support::request(
        router(support::state()),
        "GET",
        "/api/live",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status["phase"], "disabled");
    assert_eq!(status["configured"], false);
    let (code, _) = support::request(
        router(support::state()),
        "POST",
        "/api/live/connect",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(code, 409);
}

#[test]
fn live_config_accepts_disabled_defaults_and_rejects_unbounded_retries() {
    assert!(AppConfig::parse("[live]\nenabled = false").is_ok());
    assert!(AppConfig::parse("[live]\nreconnect_initial_ms = 0").is_err());
    assert!(AppConfig::parse("[live]\nenabled = true\napp_id = 0").is_err());
}
