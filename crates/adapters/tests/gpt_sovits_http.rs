#[path = "support/http.rs"]
mod http;
mod support;

use axum::{Json, Router, routing::post};
use meowlive_adapters::speech::gpt_sovits::GptSovits;
use meowlive_application::ports::speech::SpeechSynthesizer;
use serde_json::Value;

#[tokio::test]
async fn sends_explicit_wav_request_and_decodes_audio() {
    let (url, task) = http::engine(Router::new().route(
        "/tts",
        post(|Json(body): Json<Value>| async move {
            assert_eq!(body["text"], "你好");
            assert_eq!(body["ref_audio_path"], "/engine/reference.wav");
            assert_eq!(body["media_type"], "wav");
            assert_eq!(body["streaming_mode"], false);
            support::wav(&[100, -100], 1)
        }),
    ))
    .await;
    let audio = GptSovits::new(http::config(url))
        .unwrap()
        .synthesize(http::request())
        .await
        .unwrap();
    assert_eq!(audio.samples, [100, -100]);
    task.abort();
}

#[tokio::test]
async fn rejects_engine_failure_without_exposing_body() {
    let (url, task) = http::engine(Router::new().route(
        "/tts",
        post(|| async {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "private engine path /secret",
            )
        }),
    ))
    .await;
    let error = GptSovits::new(http::config(url))
        .unwrap()
        .synthesize(http::request())
        .await
        .unwrap_err();
    assert!(error.message.contains("500"));
    assert!(!error.message.contains("secret"));
    task.abort();
}
