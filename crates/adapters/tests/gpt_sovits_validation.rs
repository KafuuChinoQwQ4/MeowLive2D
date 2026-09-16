#[path = "support/http.rs"]
mod http;

use axum::{Router, routing::post};
use meowlive_adapters::speech::gpt_sovits::GptSovits;
use meowlive_application::ports::speech::SpeechSynthesizer;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[tokio::test]
async fn rejects_unknown_voices_and_invalid_text_before_contacting_the_engine() {
    let count = Arc::new(AtomicUsize::new(0));
    let counter = count.clone();
    let (url, task) = http::engine(Router::new().route(
        "/tts",
        post(move || async move {
            counter.fetch_add(1, Ordering::SeqCst);
            "unexpected engine request"
        }),
    ))
    .await;
    let adapter = GptSovits::new(http::config(url)).unwrap();
    let mut request = http::request();
    request.voice_id = "other".into();
    assert!(
        adapter
            .synthesize(request)
            .await
            .unwrap_err()
            .message
            .contains("voice")
    );
    for text in [" ".into(), "喵".repeat(501)] {
        let mut request = http::request();
        request.text = text;
        assert!(adapter.synthesize(request).await.is_err());
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
    task.abort();
}

#[test]
fn rejects_unbounded_or_ambiguous_configuration() {
    for endpoint in [
        "http://user:secret@localhost:9880",
        "http://localhost:9880?token=secret",
        "http://localhost:9880#fragment",
    ] {
        assert!(GptSovits::new(http::config(endpoint.into())).is_err());
    }
    let mut settings = http::config("http://localhost:9880".into());
    settings.timeout = Duration::ZERO;
    assert!(GptSovits::new(settings).is_err());
    let mut settings = http::config("http://localhost:9880".into());
    settings.max_audio_bytes = 0;
    assert!(GptSovits::new(settings).is_err());
    let mut settings = http::config("http://localhost:9880".into());
    settings.prompt_text = "x".repeat(65536);
    assert!(GptSovits::new(settings).is_err());
}

#[test]
fn rejects_missing_reference_and_non_http_endpoint() {
    let mut settings = http::config("file:///tmp/model".into());
    assert!(GptSovits::new(settings.clone()).is_err());
    settings.base_url = "http://localhost:9880".into();
    settings.reference_audio.clear();
    assert!(GptSovits::new(settings).is_err());
}
