//! 真实 SQL 队列经 HTTP 模型适配器驱动后台worker的跨层验收。
use crate::{config::AppConfig, state::AppState};
use axum::{Json, Router, routing::post};
use meowlive_adapters::{
    memory::{AdapterConfig, HttpMemoryAdapter},
    storage::postgres::PostgresViewerEventStore,
};
use meowlive_application::ports::{
    memory_store::MemoryStore,
    speech::{SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
    viewers::ViewerEventStore,
};
use meowlive_domain::{event::*, memory::MemoryStatus};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
struct UnusedSpeech;
impl SpeechSynthesizer for UnusedSpeech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async { panic!("memory worker must not synthesize speech") })
    }
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn http_extraction_worker_persists_evidence_vectors_and_exposes_forged_source_retries() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = Arc::new(PostgresViewerEventStore::connect(&url).await.unwrap());
    let scope = format!("worker-memory-{}", uuid::Uuid::new_v4());
    let forged = Arc::new(AtomicBool::new(false));
    let flag = forged.clone();
    let app=Router::new().route("/chat/completions",post(move |Json(req):Json<Value>|{let flag=flag.clone();async move{let sources:Value=serde_json::from_str(req["messages"][1]["content"].as_str().unwrap()).unwrap();let source=&sources[0];Json(json!({"choices":[{"message":{"content":json!({"candidates":[{"key":"preference","kind":"preference","value":"猫","explicit":true,"confidence":1.0,"evidence":[{"source":if flag.load(Ordering::Acquire){"forged-platform"}else{source["source"].as_str().unwrap()},"event_id":source["event_id"],"quote":"我喜欢猫"}],"valid_until_ms":null}]}).to_string()}}]}))}})).route("/embeddings",post(||async{Json(json!({"model":"worker-model","data":[{"index":0,"embedding":[0.25,0.75]}]}))}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let http = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let adapter = Arc::new(
        HttpMemoryAdapter::new(AdapterConfig {
            endpoint,
            api_key: "mock-only".into(),
            model: "worker-model".into(),
            dimensions: 2,
            timeout_ms: 1000,
        })
        .unwrap(),
    );
    let mut config = AppConfig::default();
    config.viewers.scope_id = scope.clone();
    config.memory.embedding_model = "worker-model".into();
    config.memory.embedding_dimensions = 2;
    let mut state = AppState::new(config, Arc::new(UnusedSpeech));
    state.memory_store = Some(store.clone());
    state.memory_extractor = Some(adapter.clone());
    state.memory_embedder = Some(adapter);
    let now = crate::viewers::utc_ms();
    let mut event = LiveEvent {
        id: "grounded".into(),
        source: "bilibili".into(),
        viewer: "测试".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "worker-test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "viewer".into(),
        }),
        occurred_at_ms: now,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: "我喜欢猫".into(),
        },
    };
    let viewer = store
        .accept_events(&scope, "session", &[event.clone()], now)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let worker = tokio::spawn(super::memory::run_memory(state.clone()));
    tokio::time::timeout(Duration::from_secs(10),async{loop{let vector_count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM memory_vectors WHERE scope_id=$1 AND model='worker-model' AND dimensions=2").bind(&scope).fetch_one(&pool).await.unwrap();if vector_count==1{break}tokio::time::sleep(Duration::from_millis(50)).await;}}).await.unwrap();
    let records = store
        .list(&scope, &viewer, 10, crate::viewers::utc_ms() as i64)
        .await
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, MemoryStatus::LongTerm);
    assert_eq!(records[0].candidate.evidence[0].event_id, "grounded");
    assert!(state.knowledge_epoch.load(Ordering::Acquire) > 0);
    forged.store(true, Ordering::Release);
    event.id = "forged".into();
    event.occurred_at_ms = crate::viewers::utc_ms();
    store
        .accept_events(&scope, "session", &[event], crate::viewers::utc_ms())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if store.status(&scope).await.unwrap().failed == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    let attempts: i32 = sqlx::query_scalar(
        "SELECT attempts FROM memory_jobs WHERE scope_id=$1 AND event_id='forged'",
    )
    .bind(&scope)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(attempts, 3);
    assert_eq!(
        store
            .list(&scope, &viewer, 10, crate::viewers::utc_ms() as i64)
            .await
            .unwrap()[0]
            .candidate
            .evidence
            .len(),
        1
    );
    state.stopping.cancel();
    tokio::time::timeout(Duration::from_secs(2), worker)
        .await
        .unwrap()
        .unwrap();
    http.abort();
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn knowledge_expiry_advances_while_real_extractor_http_is_held() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = Arc::new(PostgresViewerEventStore::connect(&url).await.unwrap());
    let scope = format!("worker-held-expiry-{}", uuid::Uuid::new_v4());
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let responded = Arc::new(AtomicBool::new(false));
    let (route_entered, route_release, route_responded) =
        (entered.clone(), release.clone(), responded.clone());
    let app = Router::new().route(
        "/chat/completions",
        post(move || {
            let entered = route_entered.clone();
            let release = route_release.clone();
            let responded = route_responded.clone();
            async move {
                entered.notify_one();
                release.notified().await;
                responded.store(true, Ordering::Release);
                Json(json!({"choices":[{"message":{"content":"{\"candidates\":[]}"}}]}))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let http = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let mut config = AppConfig::default();
    config.viewers.scope_id = scope.clone();
    let mut state = AppState::new(config, Arc::new(UnusedSpeech));
    state.memory_store = Some(store.clone());
    state.memory_extractor = Some(Arc::new(
        HttpMemoryAdapter::new(AdapterConfig {
            endpoint,
            api_key: "mock-only".into(),
            model: "held-model".into(),
            dimensions: 2,
            timeout_ms: 10000,
        })
        .unwrap(),
    ));
    let now = crate::viewers::utc_ms();
    let event = LiveEvent {
        id: "held".into(),
        source: "bilibili".into(),
        viewer: "测试".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "expiry-test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "viewer".into(),
        }),
        occurred_at_ms: now,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: "我喜欢猫".into(),
        },
    };
    store
        .accept_events(&scope, "session", &[event], now)
        .await
        .unwrap();
    let worker = tokio::spawn(super::memory::run_memory(state.clone()));
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    assert_eq!(store.status(&scope).await.unwrap().running, 1);
    let epoch = state.knowledge_epoch.load(Ordering::Acquire);
    state
        .knowledge_deadline
        .store(crate::viewers::utc_ms() as i64, Ordering::Release);
    let expiry = tokio::spawn(super::memory::run_knowledge_expiry(state.clone()));
    tokio::time::timeout(Duration::from_millis(500), async {
        while state.knowledge_epoch.load(Ordering::Acquire) == epoch {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("expiry must not wait for extractor HTTP completion");
    assert!(!responded.load(Ordering::Acquire));
    assert!(!worker.is_finished());
    assert_eq!(store.status(&scope).await.unwrap().running, 1);
    assert_eq!(state.knowledge_deadline.load(Ordering::Acquire), i64::MAX);
    state.stopping.cancel();
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), worker)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), expiry)
        .await
        .unwrap()
        .unwrap();
    http.abort();
}
