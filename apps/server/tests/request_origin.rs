mod support;
use axum::{body::Body, http::Request};
use meowlive_server::transport::http::router;
use tower::ServiceExt;

#[tokio::test]
async fn foreign_simple_post_cannot_stop_local_playback() {
    let state = support::state();
    let response = router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/stop")
                .header("origin", "https://untrusted.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert_eq!(support::status(&state).await["generation"], 0);
}

#[tokio::test]
async fn configured_panel_origin_can_stop_playback() {
    let state = support::state();
    let response = router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/stop")
                .header("origin", "http://127.0.0.1:1420")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(support::status(&state).await["generation"], 1);
}

#[tokio::test]
async fn windows_tauri_origin_can_read_status_and_control_local_server() {
    let state = support::state();
    let response = router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/stop")
                .header("origin", "http://tauri.localhost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "http://tauri.localhost"
    );
    assert_eq!(support::status(&state).await["generation"], 1);
}
