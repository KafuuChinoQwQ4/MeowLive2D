//! 真实 SQL 配合模拟模型/执行端，覆盖资料失效和完成计分的跨层路径。
mod support;
use axum::{body::Body, http::Request};
use meowlive_adapters::storage::postgres::PostgresViewerEventStore;
use meowlive_application::ports::{
    companionship::CompanionshipStore, llm::*, memory_store::*, viewers::ViewerEventStore,
};
use meowlive_domain::{event::*, memory::*};
use meowlive_server::{
    agent::run_agent, config::AppConfig, state::AppState, transport::http::router,
    worker::run_worker,
};
use serde_json::json;
use std::{future::IntoFuture, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Notify};
use tower::ServiceExt;
struct HeldModel {
    entered: Arc<Notify>,
    release: Arc<Notify>,
    seen: Arc<Mutex<Vec<DecisionRequest>>>,
}
impl LanguageModel for HeldModel {
    fn decide(&self, r: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async move {
            self.seen.lock().await.push(r.clone());
            self.entered.notify_one();
            self.release.notified().await;
            Ok(AgentDecision {
                reply_to: r.events.iter().map(|e| e.id.clone()).collect(),
                text: Some("记得你喜欢猫。".into()),
                topic: None,
            })
        })
    }
}
fn event(id: &str, now: u64) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "simulator".into(),
        viewer: "阿喵".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "simulator".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "known-viewer".into(),
        }),
        occurred_at_ms: now,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: "我喜欢猫".into(),
        },
    }
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn deleting_a_fact_cancels_inflight_generation_and_removes_future_context() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = Arc::new(PostgresViewerEventStore::connect(&url).await.unwrap());
    let scope = format!("knowledge-runtime-{}", uuid::Uuid::new_v4());
    let now = meowlive_server::viewers::utc_ms();
    let viewer = store
        .accept_events(&scope, "s", &[event("memory", now)], now)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let job = store.claim_job(&scope, now as i64).await.unwrap().unwrap();
    let candidate = MemoryCandidate {
        key: "preference".into(),
        value: "猫".into(),
        kind: MemoryKind::Preference,
        evidence: vec![Evidence::from_source(&job.sources[0], "我喜欢猫")],
        explicit: true,
        confidence: 1.0,
        valid_until_ms: None,
    };
    store
        .finish_job(&scope, &job, &[candidate], now as i64 + 1)
        .await
        .unwrap();
    let record = store
        .list(&scope, &viewer, 10, now as i64 + 2)
        .await
        .unwrap()
        .remove(0);
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut config = AppConfig::default();
    config.viewers.enabled = true;
    config.viewers.scope_id = scope.clone();
    config.auth.enabled = true;
    let mut state = AppState::with_model(
        config,
        Arc::new(FixedSpeech),
        Some(Arc::new(HeldModel {
            entered: entered.clone(),
            release: release.clone(),
            seen: seen.clone(),
        })),
    );
    // A private test credential source keeps the real authentication middleware in this path.
    let dir = std::env::temp_dir().join(format!("knowledge-auth-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(
        dir.join("admin"),
        "admin-knowledge-test-0123456789abcdef0123456789",
    )
    .unwrap();
    std::fs::write(
        dir.join("device"),
        "device-knowledge-test-0123456789abcdef012345678",
    )
    .unwrap();
    let authcfg = meowlive_server::config::AuthConfig {
        enabled: true,
        admin_token_env: String::new(),
        device_token_env: String::new(),
        admin_token_file: Some("admin".into()),
        device_token_file: Some("device".into()),
        session_lifetime_seconds: 60,
    };
    state.auth = Arc::new(
        meowlive_server::auth::AdminAuth::from_config(&authcfg, &dir.join("server.toml")).unwrap(),
    );
    state.viewer_store = Some(store.clone());
    state.memory_store = Some(store.clone());
    state.companionship_store = Some(store.clone());
    state.resources.select_voice("default").unwrap();
    let app = router(state.clone());
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/admin/session")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"token":"admin-knowledge-test-0123456789abcdef0123456789"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    use http_body_util::BodyExt;
    let body: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let token = body["token"].as_str().unwrap();
    // Agent uses the real live accept path, whose source identity is preserved.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(axum::serve(listener, app.clone()).into_future());
    let (mut control, _audio, _) =
        pair_authenticated(&base, "device-knowledge-test-0123456789abcdef012345678").await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while !state.snapshot().await.bridge_connected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    state
        .submit_events(meowlive_protocol::agent::EventBatchRequest {
            events: vec![meowlive_protocol::agent::LiveEventInput {
                id: "reply".into(),
                source: "simulator".into(),
                viewer: "阿喵".into(),
                viewer_identity: Some(meowlive_protocol::agent::ViewerIdentityInput {
                    namespace: "simulator".into(),
                    kind: meowlive_protocol::agent::ViewerIdentityKind::OpenId,
                    external_id: "known-viewer".into(),
                }),
                gift_metadata: None,
                kind: meowlive_protocol::agent::EventPayload::Chat {
                    text: "我喜欢猫".into(),
                },
            }],
        })
        .await
        .unwrap();
    let worker = tokio::spawn(run_worker(state.clone()));
    let agent = tokio::spawn(run_agent(state.clone()));
    state.resume_agent().await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), entered.notified())
        .await
        .unwrap();
    assert!(
        seen.lock().await[0]
            .memory_context
            .iter()
            .any(|v| v.contains("猫"))
    );
    assert!(
        seen.lock().await[0]
            .memory_context
            .iter()
            .all(|v| !v.contains("affinity"))
    );
    let response=app.oneshot(Request::post(format!("/api/admin/viewers/{viewer}/memories/{}",record.id)).header("content-type","application/json").header("authorization",format!("Bearer {token}")).body(Body::from(json!({"operation":"delete","expected_version":record.version,"request_key":uuid::Uuid::new_v4().to_string(),"reason":"错误记忆","value":null,"frozen":null}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), 200);
    release.notify_one();
    let stopped = support::next_json(&mut control).await;
    assert_eq!(stopped["type"], "stop");
    assert!(
        store
            .context(&scope, &[viewer.clone()], None, now as i64 + 4)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    assert!(
        state
            .snapshot()
            .await
            .speeches
            .iter()
            .all(|s| s.status != meowlive_protocol::control::SpeechStatus::Ready)
    );
    assert!(store.detail(&scope, &viewer, 20).await.unwrap().is_none());
    state.shutdown().await;
    worker.abort();
    agent.abort();
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

struct FixedSpeech;
impl meowlive_application::ports::speech::SpeechSynthesizer for FixedSpeech {
    fn synthesize(
        &self,
        _: meowlive_application::ports::speech::SynthesisRequest,
    ) -> meowlive_application::ports::speech::SynthesisFuture<'_> {
        Box::pin(async {
            Ok(meowlive_application::ports::speech::PcmAudio {
                sample_rate: 8000,
                channels: 1,
                samples: vec![0; 80],
            })
        })
    }
}
async fn pair_authenticated(
    base: &str,
    token: &str,
) -> (support::Socket, support::Socket, serde_json::Value) {
    use futures_util::SinkExt;
    use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};
    let mut r = format!("{base}/ws/control").into_client_request().unwrap();
    r.headers_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    let (mut control, _) = tokio_tungstenite::connect_async(r).await.unwrap();
    control
        .send(Message::Text(
            json!({"type":"hello","protocol_version":meowlive_protocol::PROTOCOL_VERSION})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let hello = support::next_json(&mut control).await;
    let mut r = format!(
        "{base}/ws/audio?session_id={}&bridge_id={}",
        hello["session_id"].as_str().unwrap(),
        hello["bridge_id"].as_str().unwrap()
    )
    .into_client_request()
    .unwrap();
    r.headers_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    let (audio, _) = tokio_tungstenite::connect_async(r).await.unwrap();
    (control, audio, hello)
}

mod live_support;
struct ImmediateModel;
impl LanguageModel for ImmediateModel {
    fn decide(&self, r: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async move {
            Ok(AgentDecision {
                reply_to: r.events.iter().map(|e| e.id.clone()).collect(),
                text: Some("你好，欢迎回来。".into()),
                topic: None,
            })
        })
    }
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn real_ingest_rewards_only_completed_device_receipt_and_survives_reconnect() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = Arc::new(PostgresViewerEventStore::connect(&url).await.unwrap());
    let scope = format!("receipt-runtime-{}", uuid::Uuid::new_v4());
    let now = meowlive_server::viewers::utc_ms();
    let journal_dir =
        std::env::temp_dir().join(format!("receipt-journal-runtime-{}", uuid::Uuid::new_v4()));
    let journal = Arc::new(
        meowlive_adapters::storage::receipt_journal::FileReceiptJournal::open(&journal_dir, 4096)
            .unwrap(),
    );
    use meowlive_application::ports::receipt_journal::ReceiptJournal;
    let (source, senders) = live_support::source(1, None);
    let mut config = AppConfig::default();
    config.viewers.enabled = true;
    config.viewers.scope_id = scope.clone();
    config.live.enabled = true;
    config.live.app_id = 1;
    config.agent.cooldown_ms = 1000;
    let mut state = AppState::with_services(
        config,
        Arc::new(FixedSpeech),
        Some(Arc::new(ImmediateModel)),
        Some(source),
    );
    state.receipt_journal = Some(journal.clone());
    state.viewer_store = Some(store.clone());
    state.companionship_store = Some(store.clone());
    state.memory_store = Some(store.clone());
    state.resources.select_voice("default").unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(axum::serve(listener, router(state.clone())).into_future());
    let (mut control, mut audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    state.connect_live().await.unwrap();
    let mut e = event("real", now);
    e.source = "bilibili".into();
    e.viewer_identity.as_mut().unwrap().namespace = "real-runtime".into();
    senders[0].send(Ok(Some(e.clone()))).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while state.agent_snapshot().await.events.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let viewer = store
        .resolve_viewers(&scope, &[e.clone()])
        .await
        .unwrap()
        .remove(0);
    let worker = tokio::spawn(run_worker(state.clone()));
    let agent = tokio::spawn(run_agent(state.clone()));

    state.resume_agent().await.unwrap();
    let speak = support::next_json(&mut control).await;
    assert_eq!(speak["type"], "speak");
    support::next_data(&mut audio).await;
    assert_eq!(
        store
            .detail(&scope, &viewer, 20)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        0
    );
    use futures_util::SinkExt;
    let started=json!({"type":"receipt","receipt":{"utterance_id":speak["utterance_id"],"generation":speak["generation"],"status":"started","error":null}}).to_string();
    control
        .send(tokio_tungstenite::tungstenite::Message::Text(
            started.into(),
        ))
        .await
        .unwrap();
    let receipt=json!({"type":"receipt","receipt":{"utterance_id":speak["utterance_id"],"generation":speak["generation"],"status":"completed","error":null}}).to_string();
    control
        .send(tokio_tungstenite::tungstenite::Message::Text(
            receipt.clone().into(),
        ))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while journal.pending(&scope, 10).unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        store
            .detail(&scope, &viewer, 20)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        0
    );
    control
        .send(tokio_tungstenite::tungstenite::Message::Text(
            receipt.into(),
        ))
        .await
        .unwrap();
    state.shutdown().await;
    worker.abort();
    agent.abort();
    server.abort();
    journal
        .put(&scope, "unregistered-receipt", now.saturating_sub(1))
        .unwrap();
    let recovered_journal = Arc::new(
        meowlive_adapters::storage::receipt_journal::FileReceiptJournal::open(&journal_dir, 4096)
            .unwrap(),
    );
    let mut config = AppConfig::default();
    config.viewers.scope_id = scope.clone();
    let mut recovery = AppState::new(config, Arc::new(FixedSpeech));
    recovery.receipt_journal = Some(recovered_journal.clone());
    recovery.companionship_store = Some(store.clone());
    let receipts = tokio::spawn(meowlive_server::companionship::run_receipts(
        recovery.clone(),
    ));
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if store
                .detail(&scope, &viewer, 20)
                .await
                .unwrap()
                .unwrap()
                .affinity_milli
                == 200
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        recovered_journal
            .pending(&scope, 10)
            .unwrap()
            .iter()
            .any(|r| r.speech_id == "unregistered-receipt" && r.attempts > 0)
    );
    let restarted = PostgresViewerEventStore::connect(&url).await.unwrap();
    restarted
        .complete_reply(&scope, speak["utterance_id"].as_str().unwrap(), now + 10)
        .await
        .unwrap();
    assert_eq!(
        restarted
            .detail(&scope, &viewer, 20)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        200
    );
    assert!(
        restarted
            .accept_events(&scope, "newsession", &[e], now + 20)
            .await
            .unwrap()[0]
            .duplicate
    );
    assert_eq!(
        restarted
            .detail(&scope, &viewer, 20)
            .await
            .unwrap()
            .unwrap()
            .observed_days,
        1
    );
    recovery.shutdown().await;
    receipts.abort();
    let _ = receipts.await;
    std::fs::remove_dir_all(journal_dir).unwrap();
}
