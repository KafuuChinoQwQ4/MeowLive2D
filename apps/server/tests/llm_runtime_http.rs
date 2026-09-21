mod support;

use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use meowlive_server::{
    config::{AppConfig, AuthConfig},
    transport::http::router,
};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn request(app: axum::Router, method: &str, path: &str, body: Value) -> (u16, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn owner_can_save_runtime_settings_without_echoing_search_credentials() {
    let app = router(support::state());
    let (status, snapshot) = request(app.clone(), "GET", "/api/agent/runtime", json!({})).await;
    assert_eq!(status, 200);
    let mut settings = snapshot["settings"].clone();
    settings["streaming"] = json!(false);
    let (status, saved) = request(app.clone(), "POST", "/api/agent/runtime", json!({
        "settings": settings, "search_api_key":"fixture-search-secret", "clear_search_api_key": false
    })).await;
    assert_eq!(status, 200);
    assert_eq!(saved["settings"]["streaming"], false);
    assert_eq!(saved["search_key_configured"], true);
    assert!(!saved.to_string().contains("fixture-search-secret"));
    let (_, snapshot) = request(app.clone(), "GET", "/api/agent/runtime", json!({})).await;
    assert_eq!(snapshot["settings"]["streaming"], false);
    let (status, usage) = request(app.clone(), "GET", "/api/llm/usage", json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(usage["totals"]["calls"], 0);
    let (status, activity) = request(app.clone(), "GET", "/api/agent/activity", json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(activity["phase"], "idle");
    assert_eq!(
        request(
            app,
            "GET",
            "/api/llm/usage?since_ms=2&until_ms=1",
            json!({})
        )
        .await
        .0,
        400
    );
}

#[tokio::test]
async fn runtime_routes_require_admin_when_authentication_is_enabled() {
    let config = AppConfig {
        auth: AuthConfig {
            enabled: true,
            ..AuthConfig::default()
        },
        ..AppConfig::default()
    };
    let app = router(support::state_with_config(config));
    for (method, path) in [
        ("GET", "/api/agent/runtime"),
        ("POST", "/api/agent/runtime"),
        ("GET", "/api/llm/usage"),
        ("GET", "/api/agent/activity"),
    ] {
        let (status, _) = request(app.clone(), method, path, json!({})).await;
        assert!(
            status == 401 || status == 503,
            "{method} {path} returned {status}"
        );
    }
}

#[tokio::test]
async fn runtime_management_accepts_admin_sessions_and_rejects_device_credentials() {
    use meowlive_server::auth::AdminAuth;
    use std::sync::Arc;
    let directory =
        std::env::temp_dir().join(format!("meowlive-runtime-auth-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let admin = "runtime-admin-fixture-0123456789abcdef";
    let device = "runtime-device-fixture-0123456789abcdef";
    std::fs::write(directory.join("admin.token"), admin).unwrap();
    std::fs::write(directory.join("device.token"), device).unwrap();
    let auth_config = AuthConfig {
        enabled: true,
        admin_token_env: String::new(),
        device_token_env: String::new(),
        admin_token_file: Some("admin.token".into()),
        device_token_file: Some("device.token".into()),
        ..AuthConfig::default()
    };
    let auth = AdminAuth::from_config(&auth_config, &directory.join("server.toml")).unwrap();
    let token = auth.login(admin).unwrap().token;
    let mut state = support::state_with_config(AppConfig {
        auth: auth_config,
        ..AppConfig::default()
    });
    state.auth = Arc::new(auth);
    let app = router(state);
    for path in [
        "/api/agent/runtime",
        "/api/llm/usage",
        "/api/agent/activity",
    ] {
        for (credential, expected) in [(token.as_str(), 200), (device, 401)] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header("authorization", format!("Bearer {credential}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), expected, "{path}");
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}
