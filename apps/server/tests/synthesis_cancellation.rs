mod support;
use meowlive_application::ports::speech::{
    PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest,
};
use meowlive_server::{
    config::AppConfig, state::AppState, transport::http::router, worker::run_worker,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Notify;

struct DelayedSpeech {
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
impl SpeechSynthesizer for DelayedSpeech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(PcmAudio {
                sample_rate: 32000,
                channels: 1,
                samples: vec![1; 32],
            })
        })
    }
}

#[tokio::test]
async fn stopping_during_synthesis_discards_late_audio_and_releases_worker() {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let state = AppState::new(
        AppConfig::default(),
        Arc::new(DelayedSpeech {
            entered: entered.clone(),
            release: release.clone(),
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let app = router(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let (mut control, _audio, _) = support::pair(&url).await;
    support::await_connected(&state, true).await;
    let worker = tokio::spawn(run_worker(state.clone()));
    let (_, accepted) = support::request(
        router(state.clone()),
        "POST",
        "/api/speech",
        json!({"text":"取消合成","voice_id":"default"}),
    )
    .await;
    tokio::time::timeout(std::time::Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    state.stop().await;
    release.notify_one();
    let command = support::next_json(&mut control).await;
    assert_eq!(command["type"], "stop");
    let status = support::status(&state).await;
    assert_eq!(status["speeches"][0]["id"], accepted["id"]);
    assert_eq!(status["speeches"][0]["status"], "cancelled");
    let (_, next) = support::request(
        router(state.clone()),
        "POST",
        "/api/speech",
        json!({"text":"停止后可继续播报","voice_id":"default"}),
    )
    .await;
    tokio::time::timeout(std::time::Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    release.notify_one();
    let command = support::next_json(&mut control).await;
    assert_eq!(command["type"], "speak");
    assert_eq!(command["utterance_id"], next["id"]);
    assert!(command["generation"].as_u64() > accepted["generation"].as_u64());
    worker.abort();
    server.abort();
}

#[tokio::test]
async fn disconnected_delivery_becomes_unknown_and_is_not_replayed() {
    let (state, base, server) = support::server().await;
    let (mut control, audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    let worker = tokio::spawn(run_worker(state.clone()));
    support::request(
        router(state.clone()),
        "POST",
        "/api/speech",
        json!({"text":"状态未知","voice_id":"default"}),
    )
    .await;
    assert_eq!(support::next_json(&mut control).await["type"], "speak");
    drop(audio);
    support::await_connected(&state, false).await;
    assert_eq!(
        support::status(&state).await["speeches"][0]["status"],
        "unknown"
    );
    let (_control2, _audio2, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    assert_eq!(
        support::status(&state).await["speeches"][0]["status"],
        "unknown"
    );
    worker.abort();
    server.abort();
}
