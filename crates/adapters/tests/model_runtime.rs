use axum::{Json, Router, extract::State, routing::get};
use meowlive_adapters::speech::model_synthesizer::ModelSynthesizer;
use meowlive_application::{
    ports::speech::{PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
    training::TrainingManager,
};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

struct Speech(Arc<AtomicUsize>);
impl SpeechSynthesizer for Speech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(PcmAudio {
                sample_rate: 8000,
                channels: 1,
                samples: vec![1; 8],
            })
        })
    }
}

#[tokio::test]
async fn unloaded_model_rejects_synthesis_and_can_be_enabled_again_without_poisoning() {
    async fn status(State(loaded): State<Arc<AtomicBool>>) -> Json<Value> {
        Json(
            json!({"supported":true,"state":if loaded.load(Ordering::SeqCst) {"loaded"} else {"unloaded"},"message":"controlled"}),
        )
    }
    async fn control(
        State(loaded): State<Arc<AtomicBool>>,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        loaded.store(body["enabled"].as_bool().unwrap(), Ordering::SeqCst);
        status(State(loaded)).await
    }
    let loaded = Arc::new(AtomicBool::new(false));
    let app = Router::new()
        .route("/meowlive/models", get(status).post(control))
        .route(
            "/set_gpt_weights",
            get(|| async { Json(json!({"message":"success"})) }),
        )
        .route(
            "/set_sovits_weights",
            get(|| async { Json(json!({"message":"success"})) }),
        )
        .with_state(loaded.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let synth = ModelSynthesizer::new(
        Arc::new(Speech(calls.clone())),
        Arc::new(TrainingManager::disabled()),
        &base,
        "/default.ckpt".into(),
        "/default.pth".into(),
        Duration::from_secs(2),
    )
    .unwrap();
    let request = || SynthesisRequest {
        text: "hello".into(),
        voice_id: "default".into(),
    };
    assert_eq!(synth.model_status().await.unwrap().state, "unloaded");
    assert!(
        synth
            .synthesize(request())
            .await
            .unwrap_err()
            .message
            .contains("启用")
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        synth.set_models_enabled(true).await.unwrap().state,
        "loaded"
    );
    synth.synthesize(request()).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    synth.set_models_enabled(false).await.unwrap();
    assert!(!loaded.load(Ordering::SeqCst));
    assert!(synth.synthesize(request()).await.is_err());
    synth.set_models_enabled(true).await.unwrap();
    synth.synthesize(request()).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    task.abort();
}

#[tokio::test]
async fn cancelling_enable_request_keeps_models_busy_until_engine_finishes() {
    use tokio::sync::Notify;
    #[derive(Clone)]
    struct Control {
        started: Arc<Notify>,
        release: Arc<Notify>,
    }
    async fn control(State(control): State<Control>) -> Json<Value> {
        control.started.notify_one();
        control.release.notified().await;
        Json(json!({"supported":true,"state":"loaded","message":"ready"}))
    }
    let pending = Control {
        started: Arc::new(Notify::new()),
        release: Arc::new(Notify::new()),
    };
    let app = Router::new()
        .route("/meowlive/models", axum::routing::post(control))
        .with_state(pending.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let synth = Arc::new(
        ModelSynthesizer::new(
            Arc::new(Speech(Arc::new(AtomicUsize::new(0)))),
            Arc::new(TrainingManager::disabled()),
            &base,
            "/default.ckpt".into(),
            "/default.pth".into(),
            Duration::from_secs(2),
        )
        .unwrap(),
    );
    let first = synth.clone();
    let caller = tokio::spawn(async move { first.set_models_enabled(true).await });
    pending.started.notified().await;
    caller.abort();
    let _ = caller.await;
    assert!(synth.is_busy());
    assert!(synth.set_models_enabled(false).await.is_err());
    pending.release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), async {
        while synth.is_busy() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    server.abort();
}
