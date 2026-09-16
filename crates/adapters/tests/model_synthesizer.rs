use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use meowlive_adapters::speech::model_synthesizer::ModelSynthesizer;
use meowlive_application::{
    ports::speech::{PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
    training::TrainingManager,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::sync::{Mutex, Notify};
struct FakeSpeech {
    calls: Arc<Mutex<Vec<String>>>,
    started: Arc<Notify>,
    release: Arc<Notify>,
}
impl SpeechSynthesizer for FakeSpeech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async move {
            self.calls.lock().await.push("tts-start".into());
            self.started.notify_one();
            self.release.notified().await;
            self.calls.lock().await.push("tts-done".into());
            Ok(PcmAudio {
                sample_rate: 8000,
                channels: 1,
                samples: vec![1; 100],
            })
        })
    }
}
#[tokio::test]
async fn cancelled_caller_cannot_release_half_completed_weight_transaction() {
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    async fn weights(
        State(calls): State<Arc<Mutex<Vec<String>>>>,
        Query(query): Query<std::collections::HashMap<String, String>>,
    ) -> Json<serde_json::Value> {
        calls.lock().await.push(query["weights_path"].clone());
        Json(json!({"message":"success"}))
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route("/set_gpt_weights", get(weights))
        .route("/set_sovits_weights", get(weights))
        .with_state(calls.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let synthesizer = Arc::new(
        ModelSynthesizer::new(
            Arc::new(FakeSpeech {
                calls: calls.clone(),
                started: started.clone(),
                release: release.clone(),
            }),
            Arc::new(TrainingManager::disabled()),
            &base,
            "/default.ckpt".into(),
            "/default.pth".into(),
            Duration::from_secs(2),
        )
        .unwrap(),
    );
    let first = synthesizer.clone();
    let a = tokio::spawn(async move {
        first
            .synthesize(SynthesisRequest {
                text: "first".into(),
                voice_id: "default".into(),
            })
            .await
    });
    started.notified().await;
    a.abort();
    let _ = a.await;
    assert!(synthesizer.is_busy());
    let rejected = synthesizer
        .synthesize(SynthesisRequest {
            text: "second".into(),
            voice_id: "default".into(),
        })
        .await;
    assert!(rejected.is_err());
    assert_eq!(
        *calls.lock().await,
        vec!["/default.ckpt", "/default.pth", "tts-start"]
    );
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), async {
        while synthesizer.is_busy() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let second = synthesizer.clone();
    let b = tokio::spawn(async move {
        second
            .synthesize(SynthesisRequest {
                text: "second".into(),
                voice_id: "default".into(),
            })
            .await
    });
    started.notified().await;
    release.notify_one();
    b.await.unwrap().unwrap();
    assert_eq!(
        *calls.lock().await,
        vec![
            "/default.ckpt",
            "/default.pth",
            "tts-start",
            "tts-done",
            "/default.ckpt",
            "/default.pth",
            "tts-start",
            "tts-done"
        ]
    );
    server.abort();
}
