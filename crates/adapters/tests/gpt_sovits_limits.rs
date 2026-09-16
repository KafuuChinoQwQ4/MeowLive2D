#[path = "support/http.rs"]
mod http;
#[path = "support/raw_http.rs"]
mod raw_http;
mod support;

use axum::{Router, routing::post};
use meowlive_adapters::speech::gpt_sovits::GptSovits;
use meowlive_application::ports::speech::SpeechSynthesizer;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[tokio::test]
async fn rejects_redirects_without_contacting_the_location() {
    let contacted = Arc::new(AtomicBool::new(false));
    let marker = contacted.clone();
    let (url, task) = http::engine(
        Router::new()
            .route(
                "/tts",
                post(|| async { axum::response::Redirect::temporary("/private") }),
            )
            .route(
                "/private",
                post(move || async move {
                    marker.store(true, Ordering::SeqCst);
                    support::wav(&[1], 1)
                }),
            ),
    )
    .await;
    let error = GptSovits::new(http::config(url))
        .unwrap()
        .synthesize(http::request())
        .await
        .unwrap_err();
    assert!(error.message.contains("307"));
    assert!(!contacted.load(Ordering::SeqCst));
    task.abort();
}

#[tokio::test]
async fn limits_response_bytes_even_without_a_content_length_header() {
    use tokio::io::AsyncWriteExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        raw_http::consume_request(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: audio/wav\r\n\r\n",
            )
            .await
            .unwrap();
        for _ in 0..3 {
            if stream.write_all(b"200\r\n").await.is_err() {
                return;
            }
            if stream.write_all(&[0u8; 512]).await.is_err() {
                return;
            }
            if stream.write_all(b"\r\n").await.is_err() {
                return;
            }
        }
        let _ = stream.write_all(b"0\r\n\r\n").await;
    });
    let error = GptSovits::new(http::config(url))
        .unwrap()
        .synthesize(http::request())
        .await
        .unwrap_err();
    assert!(error.message.contains("limit"));
    task.abort();
}

#[tokio::test]
async fn total_timeout_also_applies_after_response_headers_arrive() {
    use tokio::io::AsyncWriteExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        raw_http::consume_request(&mut stream).await;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nx\r\n")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
    });
    let mut settings = http::config(url);
    settings.timeout = Duration::from_millis(30);
    let error = GptSovits::new(settings)
        .unwrap()
        .synthesize(http::request())
        .await
        .unwrap_err();
    assert!(error.message.contains("timed out"));
    task.abort();
}

#[tokio::test]
async fn limits_responses_with_a_content_length_header() {
    let (url, task) =
        http::engine(Router::new().route("/tts", post(|| async { vec![0u8; 2048] }))).await;
    let error = GptSovits::new(http::config(url))
        .unwrap()
        .synthesize(http::request())
        .await
        .unwrap_err();
    assert!(error.message.contains("limit"));
    task.abort();
}

#[tokio::test]
async fn total_timeout_applies_while_waiting_for_response_headers() {
    let (url, task) = http::engine(Router::new().route(
        "/tts",
        post(|| async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            support::wav(&[1], 1)
        }),
    ))
    .await;
    let mut settings = http::config(url);
    settings.timeout = Duration::from_millis(30);
    let error = GptSovits::new(settings)
        .unwrap()
        .synthesize(http::request())
        .await
        .unwrap_err();
    assert!(error.message.contains("timed out"));
    task.abort();
}
