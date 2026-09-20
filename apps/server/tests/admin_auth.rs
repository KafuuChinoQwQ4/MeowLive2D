mod support;

use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode, header},
};
use http_body_util::BodyExt;
use meowlive_server::{
    auth::AdminAuth,
    config::{AppConfig, AuthConfig},
    transport::http::router,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tower::ServiceExt;

const ADMIN_CREDENTIAL: &str = "admin-0123456789abcdef-0123456789abcdef";
const DEVICE_CREDENTIAL: &str = "device-0123456789abcdef-0123456789abcdef";

struct AuthFiles {
    directory: PathBuf,
}

impl AuthFiles {
    fn configured(session_lifetime_seconds: u32) -> (Self, AuthConfig, AdminAuth) {
        let directory =
            std::env::temp_dir().join(format!("meowlive-auth-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(
            directory.join("admin.token"),
            format!("{ADMIN_CREDENTIAL}\n"),
        )
        .unwrap();
        std::fs::write(
            directory.join("device.token"),
            format!("{DEVICE_CREDENTIAL}\n"),
        )
        .unwrap();
        let config = AuthConfig {
            enabled: true,
            admin_token_env: String::new(),
            device_token_env: String::new(),
            admin_token_file: Some("admin.token".into()),
            device_token_file: Some("device.token".into()),
            session_lifetime_seconds,
        };
        let auth = AdminAuth::from_config(&config, &directory.join("server.toml")).unwrap();
        (Self { directory }, config, auth)
    }
}

impl Drop for AuthFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn protected_app(session_lifetime_seconds: u32) -> (AuthFiles, Router) {
    let (files, auth_config, auth) = AuthFiles::configured(session_lifetime_seconds);
    let config = AppConfig {
        auth: auth_config,
        ..AppConfig::default()
    };
    let mut state = support::state_with_config(config);
    state.auth = Arc::new(auth);
    (files, router(state))
}

async fn send(
    app: Router,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    origin: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let response = send_raw(app, method, path, bearer, origin, body).await;
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

async fn send_raw(
    app: Router,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    origin: Option<&str>,
    body: Value,
) -> Response<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(origin) = origin {
        builder = builder.header(header::ORIGIN, origin);
    }
    app.oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn login(app: Router) -> String {
    let response = send_raw(
        app,
        "POST",
        "/api/admin/session",
        None,
        None,
        json!({"token": ADMIN_CREDENTIAL}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["expires_in_seconds"], 28_800);
    body["token"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn default_owner_mode_opens_management_without_a_login() {
    let app = router(support::state());
    let (status, _) = send(app.clone(), "GET", "/api/status", None, None, json!({})).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(app.clone(), "GET", "/ws/control", None, None, json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let response = send_raw(
        app.clone(),
        "GET",
        "/api/admin/session",
        None,
        None,
        json!({}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body, json!({"enabled": false, "authenticated": true}));

    let (status, body) = send(app, "GET", "/api/admin/viewers", None, None, json!({})).await;
    // The request reaches storage instead of asking the software owner to log in.
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "viewer_store_unavailable");
}

#[tokio::test]
async fn enabled_state_starts_locked_until_credentials_are_loaded() {
    let config = AppConfig {
        auth: AuthConfig {
            enabled: true,
            ..AuthConfig::default()
        },
        ..AppConfig::default()
    };
    let app = router(support::state_with_config(config));

    let (status, body) = send(
        app.clone(),
        "GET",
        "/api/admin/session",
        None,
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"enabled": true, "authenticated": false}));

    let (status, body) = send(
        app,
        "POST",
        "/api/admin/session",
        None,
        None,
        json!({"token": ADMIN_CREDENTIAL}),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "auth_locked");
}

#[tokio::test]
async fn login_session_guards_api_and_logout_revokes_it() {
    let (_files, app) = protected_app(28_800);
    let (status, health) = send(app.clone(), "GET", "/api/health", None, None, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        health,
        json!({
            "service": "meowlive",
            "protocol_version": meowlive_protocol::PROTOCOL_VERSION,
            "bridge_connected": false
        })
    );
    assert!(health.get("speeches").is_none());
    assert!(health.get("session_id").is_none());

    let (status, _) = send(app.clone(), "GET", "/api/status", None, None, json!({})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let token = login(app.clone()).await;
    assert_ne!(token, ADMIN_CREDENTIAL);
    assert!(token.len() >= 64);

    let response = send_raw(
        app.clone(),
        "GET",
        "/api/admin/session",
        Some(&token),
        None,
        json!({}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body, json!({"enabled": true, "authenticated": true}));

    let (status, _) = send(
        app.clone(),
        "GET",
        "/api/status",
        Some(&token),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let response = send_raw(
        app.clone(),
        "DELETE",
        "/api/admin/session",
        Some(&token),
        None,
        json!({}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );

    let (status, _) = send(app, "GET", "/api/status", Some(&token), None, json!({})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn session_capacity_evicts_the_oldest_token() {
    let (_files, app) = protected_app(28_800);
    let mut sessions = Vec::new();
    for _ in 0..17 {
        sessions.push(login(app.clone()).await);
    }
    let (status, _) = send(
        app.clone(),
        "GET",
        "/api/status",
        Some(&sessions[0]),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send(
        app,
        "GET",
        "/api/status",
        Some(sessions.last().unwrap()),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn admin_and_device_credentials_cannot_cross_roles() {
    let (_files, app) = protected_app(28_800);
    let admin_session = login(app.clone()).await;

    let (status, _) = send(
        app.clone(),
        "GET",
        "/api/status",
        Some(DEVICE_CREDENTIAL),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = send(
        app.clone(),
        "GET",
        "/ws/control",
        Some(&admin_session),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = send(
        app.clone(),
        "GET",
        "/ws/control",
        Some(DEVICE_CREDENTIAL),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = send(
        app,
        "POST",
        "/api/admin/session",
        None,
        None,
        json!({"token": DEVICE_CREDENTIAL}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sessions_expire_and_failed_logins_are_rate_limited() {
    let (_files, app) = protected_app(1);
    let (status, body) = send(
        app.clone(),
        "POST",
        "/api/admin/session",
        None,
        None,
        json!({"token": ADMIN_CREDENTIAL}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["expires_in_seconds"], 1);
    let token = body["token"].as_str().unwrap().to_owned();
    tokio::time::sleep(Duration::from_millis(1_050)).await;
    let (status, _) = send(
        app.clone(),
        "GET",
        "/api/status",
        Some(&token),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    for _ in 0..5 {
        let (status, _) = send(
            app.clone(),
            "POST",
            "/api/admin/session",
            None,
            None,
            json!({"token": "wrong-credential"}),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, body) = send(
        app,
        "POST",
        "/api/admin/session",
        None,
        None,
        json!({"token": ADMIN_CREDENTIAL}),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["code"], "login_rate_limited");
}

#[tokio::test]
async fn origin_validation_and_cors_remain_independent_of_authentication() {
    let (_files, app) = protected_app(28_800);
    let token = login(app.clone()).await;
    let (status, body) = send(
        app.clone(),
        "GET",
        "/api/status",
        Some(&token),
        Some("https://untrusted.example"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "invalid_origin");

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/admin/session")
                .header(header::ORIGIN, "http://127.0.0.1:1420")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "DELETE")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("authorization")
    );
}

#[test]
fn credential_files_are_relative_and_credentials_and_session_lifetime_are_bounded() {
    let (files, mut config, _) = AuthFiles::configured(28_800);
    config.session_lifetime_seconds = 28_801;
    assert!(config.validate().is_err());
    config.session_lifetime_seconds = 0;
    assert!(config.validate().is_err());

    config.session_lifetime_seconds = 60;
    config.device_token_file = Some("admin.token".into());
    assert!(AdminAuth::from_config(&config, &files.directory.join("server.toml")).is_err());
    config.device_token_file = Some("device.token".into());

    let directory =
        std::env::temp_dir().join(format!("meowlive-auth-short-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("admin.token"), "too-short").unwrap();
    std::fs::write(directory.join("device.token"), DEVICE_CREDENTIAL).unwrap();
    config.session_lifetime_seconds = 60;
    let error = AdminAuth::from_config(&config, &directory.join("server.toml")).unwrap_err();
    assert!(!error.contains("too-short"));
    std::fs::write(directory.join("admin.token"), "a".repeat(1025)).unwrap();
    let error = AdminAuth::from_config(&config, &directory.join("server.toml")).unwrap_err();
    assert!(!error.contains(&"a".repeat(32)));
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn companionship_management_is_never_anonymous() {
    let (_files, app) = protected_app(28_800);
    for (method, path) in [
        (
            "GET",
            "/api/admin/viewers/00000000-0000-0000-0000-000000000001",
        ),
        (
            "POST",
            "/api/admin/viewers/00000000-0000-0000-0000-000000000001/adjust",
        ),
        (
            "POST",
            "/api/admin/viewers/00000000-0000-0000-0000-000000000001/reverse",
        ),
        (
            "POST",
            "/api/admin/viewers/00000000-0000-0000-0000-000000000001/gifts/confirm",
        ),
    ] {
        let (status, body) = send(app.clone(), method, path, None, None, json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(body["code"], "authentication_required");
    }
}

#[tokio::test]
async fn companionship_reports_disabled_store_without_exposing_internal_data() {
    let (_files, app) = protected_app(28_800);
    let token = login(app.clone()).await;
    let (status, body) = send(
        app,
        "GET",
        "/api/admin/viewers/00000000-0000-0000-0000-000000000001",
        Some(&token),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "companionship_unavailable");
}

#[tokio::test]
async fn memory_graph_merge_and_recovery_routes_require_admin_session() {
    let (_files, app) = protected_app(28_800);
    for (method, path) in [
        (
            "GET",
            "/api/admin/viewers/00000000-0000-0000-0000-000000000001/memories",
        ),
        (
            "POST",
            "/api/admin/viewers/00000000-0000-0000-0000-000000000001/memories/00000000-0000-0000-0000-000000000002",
        ),
        ("POST", "/api/admin/memories/retry"),
        ("POST", "/api/admin/memories/rebuild-vectors"),
        ("POST", "/api/admin/relationships"),
        ("POST", "/api/admin/graph/rebuild"),
        ("POST", "/api/admin/viewers/merge/preview"),
        ("POST", "/api/admin/viewers/merge/apply"),
    ] {
        let (status, _) = send(app.clone(), method, path, None, None, json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
    }
    let token = login(app.clone()).await;
    for path in [
        "/api/admin/memories/status",
        "/api/admin/graph/status",
        "/api/admin/viewers/00000000-0000-0000-0000-000000000001/relationships",
    ] {
        let (status, _) = send(app.clone(), "GET", path, Some(&token), None, json!({})).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
    }
    let (status,_)=send(app,"POST","/api/admin/viewers/merge/apply",Some(&token),None,json!({"source_viewer_id":"00000000-0000-0000-0000-000000000001","target_viewer_id":"00000000-0000-0000-0000-000000000002","expected_revision":0,"fingerprint":"a".repeat(64),"request_key":uuid::Uuid::new_v4().to_string(),"reason":"same person","confirmed":false})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
