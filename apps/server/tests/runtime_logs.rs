mod support;
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use meowlive_server::transport::http::router;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn call(app: axum::Router, method: &str, uri: &str, body: Value) -> (u16, Value, bool) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let no_store = response
        .headers()
        .get("cache-control")
        .is_some_and(|h| h == "no-store");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        no_store,
    )
}

#[tokio::test]
async fn log_api_accepts_only_safe_codes_and_filters_without_logging_queries() {
    let app = router(support::state());
    let (status, _, _) = call(
        app.clone(),
        "POST",
        "/api/logs",
        json!({"code":"browser_error"}),
    )
    .await;
    assert_eq!(status, 201);
    for body in [
        json!({"code":"secret=TOKEN"}),
        json!({"code":"browser_error", "message":"secret=TOKEN"}),
    ] {
        assert_eq!(call(app.clone(), "POST", "/api/logs", body).await.0, 400);
    }
    let (status, first, no_store) = call(
        app.clone(),
        "GET",
        "/api/logs?level=error&source=browser&category=runtime&limit=1",
        Value::Null,
    )
    .await;
    assert_eq!(status, 200);
    assert!(no_store);
    assert_eq!(first["entries"].as_array().unwrap().len(), 1);
    assert_eq!(first["entries"][0]["code"], "browser_error");
    assert!(!first.to_string().contains("TOKEN"));
    let second = call(app.clone(), "GET", "/api/logs", Value::Null).await.1;
    assert_eq!(second["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        call(app.clone(), "GET", "/api/logs?limit=1000", Value::Null)
            .await
            .0,
        200
    );
    for query in ["limit=0", "limit=1001", "level=trace", "unknown=x"] {
        assert_eq!(
            call(
                app.clone(),
                "GET",
                &format!("/api/logs?{query}"),
                Value::Null
            )
            .await
            .0,
            400
        );
    }
}

#[tokio::test]
async fn log_routes_are_admin_protected() {
    let mut config = meowlive_server::config::AppConfig::default();
    config.auth.enabled = true;
    let app = router(support::state_with_config(config));
    for method in ["GET", "POST"] {
        let status = call(
            app.clone(),
            method,
            "/api/logs",
            json!({"code":"browser_error"}),
        )
        .await
        .0;
        assert!(status == 401 || status == 503);
    }
}

#[tokio::test]
async fn failed_http_requests_log_static_category_without_request_content() {
    let app = router(support::state());
    assert_eq!(
        call(
            app.clone(),
            "POST",
            "/api/speech?secret=TOKEN",
            json!({"secret":"PASSWORD"})
        )
        .await
        .0,
        400
    );
    let logs = call(app, "GET", "/api/logs?category=speech", Value::Null)
        .await
        .1;
    assert_eq!(logs["entries"][0]["code"], "http_client_error");
    assert!(!logs.to_string().contains("TOKEN"));
    assert!(!logs.to_string().contains("PASSWORD"));
}

#[test]
fn disk_logs_restore_safe_events_and_rotate_with_bounded_memory() {
    use meowlive_server::logs::{LogEvent, RuntimeLogStore};
    let directory = std::env::temp_dir().join(format!("runtime-logs-{}", uuid::Uuid::new_v4()));
    let path = directory.join("events.jsonl");
    let store = RuntimeLogStore::open(&path);
    let first = store.record(LogEvent::LlmFailed);
    assert!(chrono::DateTime::parse_from_rfc3339(&first.timestamp).is_ok());
    let mut tampered = serde_json::to_value(&first).unwrap();
    tampered["summary"] = json!("secret=TOKEN");
    tampered["source"] = json!("secret=TOKEN");
    std::fs::write(&path, format!("{tampered}\n")).unwrap();
    drop(store);
    let store = RuntimeLogStore::open(&path);
    let restored = store.list(None, None, None, None, 10);
    assert_eq!(restored.entries.len(), 1);
    assert_eq!(restored.entries[0].summary, "模型调用失败");
    assert!(!serde_json::to_string(&restored).unwrap().contains("TOKEN"));
    for _ in 0..5001 {
        store.record(LogEvent::LlmCompleted);
    }
    let bounded = store.list(None, None, None, None, 6000);
    assert_eq!(bounded.entries.len(), 5000);
    assert!(bounded.truncated);
    drop(store);
    let reopened = RuntimeLogStore::open(&path);
    let logs = reopened.list(None, None, None, None, 6000);
    assert!(logs.storage_available);
    assert!((2500..=5000).contains(&logs.entries.len()));
    assert!(std::fs::metadata(&path).unwrap().len() < 2 * 1024 * 1024);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unavailable_log_disk_keeps_in_memory_entries() {
    use meowlive_server::logs::{LogEvent, RuntimeLogStore};
    let directory = std::env::temp_dir().join(format!("runtime-logs-{}", uuid::Uuid::new_v4()));
    std::fs::write(&directory, "occupied").unwrap();
    let store = RuntimeLogStore::open(directory.join("events.jsonl"));
    store.record(LogEvent::ServerStarted);
    let logs = store.list(None, None, None, None, 10);
    assert!(!logs.storage_available);
    assert_eq!(logs.entries.len(), 1);
    std::fs::remove_file(directory).unwrap();
}

#[test]
fn partial_disk_tail_does_not_swallow_following_event() {
    use meowlive_server::logs::{LogEvent, RuntimeLogStore};
    let dir = std::env::temp_dir().join(format!("runtime-logs-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("events.jsonl");
    std::fs::write(&path, "{\"partial\":").unwrap();
    RuntimeLogStore::open(&path).record(LogEvent::ServerStarted);
    assert_eq!(
        RuntimeLogStore::open(&path)
            .list(None, None, None, None, 10)
            .entries
            .len(),
        1
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn log_filters_are_bounded_and_case_insensitive() {
    let app = router(support::state());
    call(
        app.clone(),
        "POST",
        "/api/logs",
        json!({"code":"browser_error"}),
    )
    .await;
    let logs = call(app.clone(), "GET", "/api/logs?query=BROWSER", Value::Null)
        .await
        .1;
    assert_eq!(logs["entries"].as_array().unwrap().len(), 1);
    for query in [
        format!("query={}", "a".repeat(513)),
        format!("source={}", "a".repeat(65)),
        format!("category={}", "a".repeat(65)),
    ] {
        assert_eq!(
            call(
                app.clone(),
                "GET",
                &format!("/api/logs?{query}"),
                Value::Null
            )
            .await
            .0,
            400
        );
    }
}

#[test]
fn repeated_background_errors_are_bounded_and_large_disk_files_remain_untouched() {
    use meowlive_server::logs::{LogEvent, RuntimeLogStore};
    let store = RuntimeLogStore::memory();
    for _ in 0..20 {
        store.record_throttled(LogEvent::ReceiptFailed);
    }
    assert_eq!(
        store.list(None, None, None, None, usize::MAX).entries.len(),
        1
    );
    let dir = std::env::temp_dir().join(format!("runtime-logs-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("events.jsonl");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(3 * 1024 * 1024).unwrap();
    drop(file);
    let store = RuntimeLogStore::open(&path);
    store.record(LogEvent::ServerStarted);
    assert!(!store.list(None, None, None, None, 10).storage_available);
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 3 * 1024 * 1024);
    assert!(!path.with_extension("previous.jsonl").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
