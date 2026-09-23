mod support;

use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use meowlive_protocol::agent_observability::AgentTraceEvent;
use meowlive_server::{config::AppConfig, transport::http::router};
use serde_json::Value;
use tower::ServiceExt;

async fn get(app: axum::Router, path: &str) -> (u16, Option<String>, Value) {
    let response = app
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, cache, value)
}

#[tokio::test]
async fn scheduler_list_and_detail_are_bounded_no_store_queries() {
    let state = support::state();
    let trace_id = state.agent_observability.start_trace(
        "live_events",
        vec![AgentTraceEvent {
            id: "event-1".into(),
            kind: "chat".into(),
            viewer: "小猫".into(),
            summary: "晚上好".into(),
        }],
    );
    let app = router(state);
    let (status, cache, scheduler) = get(app.clone(), "/api/agent/scheduler").await;
    assert_eq!(status, 200);
    assert_eq!(cache.as_deref(), Some("no-store"));
    assert_eq!(scheduler["block_reason"], "paused");
    let (status, cache, list) = get(app.clone(), "/api/agent/traces?limit=1").await;
    assert_eq!(status, 200);
    assert_eq!(cache.as_deref(), Some("no-store"));
    assert_eq!(list["traces"][0]["id"], trace_id);
    let (status, cache, trace) = get(app.clone(), &format!("/api/agent/traces/{trace_id}")).await;
    assert_eq!(status, 200);
    assert_eq!(cache.as_deref(), Some("no-store"));
    assert_eq!(trace["events"][0]["summary"], "晚上好");
    for path in [
        "/api/agent/traces?limit=0",
        "/api/agent/traces?limit=101",
        "/api/agent/traces?before_ms=nope",
        "/api/agent/traces?unknown=1",
    ] {
        assert_eq!(get(app.clone(), path).await.0, 400, "{path}");
    }
    let (status, _, missing) = get(app.clone(), "/api/agent/traces/not-found").await;
    assert_eq!(status, 404);
    assert_eq!(missing["code"], "agent_trace_not_found");
    assert_eq!(get(app, "/api/agent/traces/invalid%2Fpath").await.0, 400);
}

#[tokio::test]
async fn observability_routes_require_admin_authentication_when_enabled() {
    let config = AppConfig {
        auth: meowlive_server::config::AuthConfig {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let app = router(support::state_with_config(config));
    for path in [
        "/api/agent/scheduler",
        "/api/agent/traces",
        "/api/agent/traces/trace-1",
    ] {
        let status = get(app.clone(), path).await.0;
        assert!(status == 401 || status == 503, "{path}: {status}");
    }
}
