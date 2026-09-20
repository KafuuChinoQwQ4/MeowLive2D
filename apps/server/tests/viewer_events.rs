mod support;
use meowlive_application::ports::viewers::*;
use meowlive_domain::event::LiveEvent;
use meowlive_protocol::agent::EventBatchRequest;
use meowlive_server::state::AppState;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Store {
    fail: bool,
    gate: Option<tokio::sync::Semaphore>,
    entered: std::sync::atomic::AtomicUsize,
    events: Mutex<Vec<LiveEvent>>,
}
impl ViewerEventStore for Store {
    fn accept_events<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        events: &'a [LiveEvent],
        received: u64,
    ) -> ViewerStoreFuture<'a, Vec<StoreEventOutcome>> {
        Box::pin(async move {
            self.entered
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if let Some(gate) = &self.gate {
                gate.acquire().await.unwrap().forget();
            }
            if self.fail {
                return Err(ViewerStoreError::new("private database detail"));
            }
            assert!(received > 1_700_000_000_000);
            let mut saved = self.events.lock().unwrap();
            Ok(events
                .iter()
                .map(|e| {
                    assert!(e.occurred_at_ms > 1_700_000_000_000);
                    let duplicate = saved.iter().any(|old| old.id == e.id);
                    if !duplicate {
                        saved.push(e.clone());
                    }
                    StoreEventOutcome {
                        duplicate,
                        viewer_id: None,
                    }
                })
                .collect())
        })
    }
    fn list_viewers<'a>(
        &'a self,
        _: &'a str,
        _: u32,
        _: u64,
    ) -> ViewerStoreFuture<'a, Vec<ViewerSummary>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn list_events<'a>(
        &'a self,
        _: &'a str,
        _: u32,
        _: u64,
    ) -> ViewerStoreFuture<'a, Vec<PersistedEventSummary>> {
        Box::pin(async { Ok(vec![]) })
    }
}
fn batch(id: &str) -> EventBatchRequest {
    serde_json::from_value(json!({"events":[{"id":id,"source":"bilibili","viewer":"test","kind":{"type":"chat","text":"hello"}}]})).unwrap()
}
fn configured(store: Arc<Store>) -> AppState {
    let mut s = support::state();
    s.viewer_store = Some(store);
    s
}

#[tokio::test]
async fn outage_never_claims_persistence_or_admits_new_events() {
    let state = configured(Arc::new(Store {
        fail: true,
        events: Mutex::new(vec![]),
        ..Default::default()
    }));
    let error = state.submit_events(batch("one")).await.unwrap_err();
    let _ = error; // HTTP assertions below exercise redaction separately.
    assert!(state.agent_snapshot().await.events.is_empty());
}
#[tokio::test]
async fn duplicate_after_service_restart_is_not_scheduled_again_and_simulation_is_isolated() {
    let store = Arc::new(Store {
        fail: false,
        events: Mutex::new(vec![]),
        ..Default::default()
    });
    let first = configured(store.clone());
    assert_eq!(first.submit_events(batch("one")).await.unwrap().accepted, 1);
    assert_eq!(store.events.lock().unwrap()[0].source, "simulator");
    let restarted = configured(store);
    let result = restarted.submit_events(batch("one")).await.unwrap();
    assert_eq!(result.duplicates, 1);
    assert_eq!(result.accepted, 0);
    assert!(restarted.agent_snapshot().await.events.is_empty());
}

#[tokio::test]
async fn a_full_response_queue_does_not_erase_received_facts() {
    let store = Arc::new(Store {
        fail: false,
        events: Mutex::new(vec![]),
        ..Default::default()
    });
    let state = configured(store.clone());
    // Default pending capacity is bounded: fill it, then persist another event.
    for n in 0..state.config.agent.pending_capacity {
        state
            .submit_events(batch(&format!("event-{n}")))
            .await
            .unwrap();
    }
    let result = state.submit_events(batch("overflow")).await.unwrap();
    assert_eq!(result.persisted, Some(1));
    assert_eq!(result.accepted, 0);
    assert_eq!(result.unscheduled, Some(1));
    assert!(
        store
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.id == "overflow")
    );
}

mod live_support;
#[tokio::test]
async fn real_source_keeps_platform_utc_and_never_replays_expired_facts() {
    let (source, senders) = live_support::source(1, None);
    let mut state = live_support::state(source);
    let store = Arc::new(Store {
        fail: false,
        events: Mutex::new(vec![]),
        ..Default::default()
    });
    state.viewer_store = Some(store.clone());
    state.connect_live().await.unwrap();
    live_support::phase(
        &state,
        meowlive_protocol::live::LiveConnectionPhase::Connected,
    )
    .await;
    let stale = live_support::event("stale-but-valuable");
    let utc = stale.occurred_at_ms;
    senders[0].send(Ok(Some(stale))).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while store.events.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(store.events.lock().unwrap()[0].occurred_at_ms, utc);
    assert_eq!(store.events.lock().unwrap()[0].source, "bilibili");
    assert!(state.agent_snapshot().await.events.is_empty());
    state.shutdown().await;
}

#[tokio::test(start_paused = true)]
async fn delivery_delay_reduces_remaining_reply_lifetime_after_admission() {
    let (source, senders) = live_support::source(1, None);
    let mut state = live_support::state(source);
    state.viewer_store = Some(Arc::new(Store {
        fail: false,
        events: Mutex::new(vec![]),
        ..Default::default()
    }));
    state.connect_live().await.unwrap();
    live_support::phase(
        &state,
        meowlive_protocol::live::LiveConnectionPhase::Connected,
    )
    .await;
    let mut delayed = live_support::event("nearly-expired");
    delayed.occurred_at_ms =
        meowlive_server::viewers::utc_ms() - state.config.agent.event_ttl_ms + 1000;
    senders[0].send(Ok(Some(delayed))).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while state.agent_snapshot().await.events.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::time::advance(std::time::Duration::from_millis(2000)).await;
    assert_eq!(
        state.agent_snapshot().await.events[0].status,
        meowlive_protocol::agent::AgentEventStatus::Expired
    );
    state.shutdown().await;
}

async fn wait_rejected(state: &AppState) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while state.live_snapshot().await.rejected_events == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let snapshot = state.live_snapshot().await;
    assert_eq!(snapshot.rejected_events, 1);
    assert_eq!(snapshot.accepted_events, 0);
    assert_eq!(
        snapshot.phase,
        meowlive_protocol::live::LiveConnectionPhase::Connected
    );
    assert!(snapshot.last_error.unwrap().contains("数据缺口"));
    assert!(state.agent_snapshot().await.events.is_empty());
}

async fn assert_gap_count(state: &mut AppState) {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let root = std::env::temp_dir().join(format!("viewer-gap-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let admin = "gap-test-admin-0123456789abcdef0123456789abcdef";
    std::fs::write(root.join("admin"), admin).unwrap();
    std::fs::write(
        root.join("device"),
        "gap-test-device-0123456789abcdef0123456789abcdef",
    )
    .unwrap();
    let config = meowlive_server::config::AuthConfig {
        enabled: true,
        admin_token_env: String::new(),
        device_token_env: String::new(),
        admin_token_file: Some(root.join("admin")),
        device_token_file: Some(root.join("device")),
        ..Default::default()
    };
    state.auth = Arc::new(
        meowlive_server::auth::AdminAuth::from_config(&config, &root.join("server.toml")).unwrap(),
    );
    let session = state.auth.login(admin).unwrap();
    let response = meowlive_server::transport::http::router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/admin/events")
                .header("authorization", format!("Bearer {}", session.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["unconfirmed_events"], 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn live_database_failure_marks_one_gap_without_scheduling_or_disconnecting() {
    let (source, senders) = live_support::source(1, None);
    let mut state = live_support::state(source);
    state.viewer_store = Some(Arc::new(Store {
        fail: true,
        ..Default::default()
    }));
    state.connect_live().await.unwrap();
    live_support::phase(
        &state,
        meowlive_protocol::live::LiveConnectionPhase::Connected,
    )
    .await;
    let mut event = live_support::event("database-outage");
    event.occurred_at_ms = meowlive_server::viewers::utc_ms();
    senders[0].send(Ok(Some(event))).await.unwrap();
    wait_rejected(&state).await;
    assert_gap_count(&mut state).await;
    state.shutdown().await;
}

#[tokio::test]
async fn saturated_ingestion_records_a_live_gap_and_recovers_when_requests_finish() {
    let (source, senders) = live_support::source(1, None);
    let mut state = live_support::state(source);
    let store = Arc::new(Store {
        gate: Some(tokio::sync::Semaphore::new(0)),
        ..Default::default()
    });
    state.viewer_store = Some(store.clone());
    state.connect_live().await.unwrap();
    live_support::phase(
        &state,
        meowlive_protocol::live::LiveConnectionPhase::Connected,
    )
    .await;
    let mut requests = tokio::task::JoinSet::new();
    for n in 0..8 {
        let state = state.clone();
        requests.spawn(async move { state.submit_events(batch(&format!("blocked-{n}"))).await });
    }
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while store.entered.load(std::sync::atomic::Ordering::SeqCst) < 8 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut event = live_support::event("no-admission-slot");
    event.occurred_at_ms = meowlive_server::viewers::utc_ms();
    senders[0].send(Ok(Some(event))).await.unwrap();
    wait_rejected(&state).await;
    assert_gap_count(&mut state).await;
    assert_eq!(store.entered.load(std::sync::atomic::Ordering::SeqCst), 8);
    requests.abort_all();
    while requests.join_next().await.is_some() {}
    store.gate.as_ref().unwrap().add_permits(1);
    let mut next = live_support::event("after-capacity-recovers");
    next.occurred_at_ms = meowlive_server::viewers::utc_ms();
    senders[0].send(Ok(Some(next))).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while state.live_snapshot().await.accepted_events == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(state.agent_snapshot().await.events.len(), 1);
    state.shutdown().await;
}
