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
async fn sends_complete_long_superchat_and_reply_to_engine() {
    let expected = format!(
        "感谢小猫的30元SC。{}。{}",
        "原".repeat(500),
        "答".repeat(500)
    );
    let expected_body = expected.clone();
    let mut samples = vec![1_234; 32_000 * 240];
    samples[0] = 100;
    *samples.last_mut().unwrap() = -100;
    let wav = axum::body::Bytes::from(support::wav(&samples, 1));
    assert!(wav.len() > 8 * 1024 * 1024);
    assert!(samples.len() > 5_760_000);
    let (url, task) = http::engine(Router::new().route(
        "/tts",
        post(move |Json(body): Json<Value>| {
            let expected_body = expected_body.clone();
            let wav = wav.clone();
            async move {
                assert_eq!(body["text"], expected_body);
                wav
            }
        }),
    ))
    .await;
    let mut request = http::request();
    request.text = expected;
    let mut config = http::config(url);
    config.max_audio_bytes = 32 * 1024 * 1024;
    config.timeout = std::time::Duration::from_secs(300);
    let audio = GptSovits::new(config)
        .unwrap()
        .synthesize(request)
        .await
        .unwrap();
    assert_eq!(audio.sample_rate, 32_000);
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.samples, samples);
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

#[tokio::test]
async fn rejects_silent_synthesis_instead_of_reporting_successful_playback() {
    let (url, task) =
        http::engine(Router::new().route("/tts", post(|| async { support::wav(&[0; 8000], 1) })))
            .await;
    let mut config = http::config(url);
    config.max_audio_bytes = 32 * 1024;
    let result = GptSovits::new(config)
        .unwrap()
        .synthesize(http::request())
        .await;
    assert!(
        result.is_err(),
        "silent synthesis must not be delivered as successful audio"
    );
    let error = result.unwrap_err();
    assert!(error.message.contains("静音"));
    assert!(error.message.contains("参考文本"));
    task.abort();
}
