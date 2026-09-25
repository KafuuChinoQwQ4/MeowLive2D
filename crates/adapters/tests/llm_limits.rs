mod llm_support;

use axum::{Router, http::StatusCode, routing::post};
use meowlive_adapters::llm::openai_compatible::OpenAiCompatible;
use meowlive_application::ports::llm::LanguageModel;
use std::time::Duration;

#[test]
fn rejects_unsafe_or_out_of_bounds_configuration_and_hides_key() {
    for base_url in [
        "file:///tmp/model",
        " http://localhost:1234",
        "http://@localhost:1234",
        "http://user:secret@localhost:1234",
        "http://localhost:1234?token=secret",
        "http://localhost:1234#secret",
    ] {
        assert!(OpenAiCompatible::new(llm_support::config(base_url.into())).is_err());
    }
    for timeout in [
        Duration::ZERO,
        Duration::from_millis(999),
        Duration::from_secs(121),
    ] {
        let mut config = llm_support::config("http://localhost:1234".into());
        config.timeout = timeout;
        assert!(OpenAiCompatible::new(config).is_err());
    }
    for bytes in [1023, 1024 * 1024 + 1] {
        let mut config = llm_support::config("http://localhost:1234".into());
        config.max_response_bytes = bytes;
        assert!(OpenAiCompatible::new(config).is_err());
    }
    for tokens in [63, 65537] {
        let mut config = llm_support::config("http://localhost:1234".into());
        config.max_tokens = tokens;
        assert!(OpenAiCompatible::new(config).is_err());
    }
    for model in [" ".to_owned(), "x".repeat(129), "bad\nmodel".to_owned()] {
        let mut config = llm_support::config("http://localhost:1234".into());
        config.model = model;
        assert!(OpenAiCompatible::new(config).is_err());
    }
    for key in [" ", "secret\r\nforwarded: true"] {
        let mut config = llm_support::config("http://localhost:1234".into());
        config.api_key = Some(key.into());
        assert!(OpenAiCompatible::new(config).is_err());
    }
    let config = llm_support::config("http://localhost:1234".into());
    assert!(!format!("{config:?}").contains("test-token"));
}

#[tokio::test]
async fn rejects_invalid_bounded_request_before_network_io() {
    let adapter = OpenAiCompatible::new(llm_support::config("http://127.0.0.1:9".into())).unwrap();
    let mut requests = Vec::new();
    let mut request = llm_support::request(Vec::new());
    request.persona = "x".repeat(2001);
    requests.push(request);
    let mut request = llm_support::request(Vec::new());
    request.system_prompt = "x".repeat(4001);
    requests.push(request);
    let mut request = llm_support::request(Vec::new());
    request.topic = "x".repeat(201);
    requests.push(request);
    let mut request = llm_support::request(Vec::new());
    request.topic = "bad\rtopic".into();
    requests.push(request);
    let mut request = llm_support::request(Vec::new());
    request.events = (0..17)
        .map(|index| llm_support::chat(&format!("chat-{index}"), "hi"))
        .collect();
    requests.push(request);
    let mut request = llm_support::request(Vec::new());
    request.history[0].user = "x".repeat(4001);
    requests.push(request);
    for request in requests {
        let error = adapter.decide(request).await.unwrap_err();
        assert!(!error.retryable);
        assert!(error.message.contains("request"));
    }
}

#[tokio::test]
async fn bounds_declared_and_streamed_response_bodies() {
    let (url, task) = llm_support::server(
        Router::new().route("/chat/completions", post(|| async { vec![b'x'; 2048] })),
    )
    .await;
    let mut config = llm_support::config(url);
    config.max_response_bytes = 1024;
    let error = OpenAiCompatible::new(config)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap_err();
    assert!(error.message.contains("limit"));
    task.abort();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        let (mut stream, _) = listener.accept().await.unwrap();
        crate::raw_http::consume_request(&mut stream).await;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
            .await
            .unwrap();
        for _ in 0..3 {
            if stream.write_all(b"200\r\n").await.is_err() {
                return;
            }
            if stream.write_all(&[b'x'; 512]).await.is_err() {
                return;
            }
            if stream.write_all(b"\r\n").await.is_err() {
                return;
            }
        }
    });
    let mut config = llm_support::config(url);
    config.max_response_bytes = 1024;
    let error = OpenAiCompatible::new(config)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap_err();
    assert!(error.message.contains("limit"));
    task.abort();
}

#[tokio::test]
async fn classifies_only_temporary_transport_and_status_failures_as_retryable() {
    for (status, retryable) in [
        (StatusCode::TOO_MANY_REQUESTS, true),
        (StatusCode::BAD_GATEWAY, true),
        (StatusCode::UNAUTHORIZED, false),
    ] {
        let (url, task) = llm_support::server(Router::new().route(
            "/chat/completions",
            post(move || async move { (status, "secret body and token test-token") }),
        ))
        .await;
        let error = OpenAiCompatible::new(llm_support::config(url))
            .unwrap()
            .decide(llm_support::request(Vec::new()))
            .await
            .unwrap_err();
        assert_eq!(error.retryable, retryable);
        assert!(error.message.contains(status.as_str()));
        assert!(!error.message.contains("secret body"));
        assert!(!error.message.contains("test-token"));
        task.abort();
    }
}

#[tokio::test]
async fn classifies_connection_failure_as_retryable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let error = OpenAiCompatible::new(llm_support::config(url))
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap_err();
    assert!(error.retryable);
    assert!(error.message.contains("connect"));
}

#[tokio::test]
async fn total_timeout_includes_streamed_response_body() {
    use tokio::io::AsyncWriteExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        crate::raw_http::consume_request(&mut stream).await;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nx\r\n")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
    });
    let mut config = llm_support::config(url);
    config.timeout = Duration::from_secs(1);
    let error = OpenAiCompatible::new(config)
        .unwrap()
        .decide(llm_support::request(Vec::new()))
        .await
        .unwrap_err();
    assert!(error.retryable);
    assert!(error.message.contains("timed out"));
    task.abort();
}

#[path = "support/raw_http.rs"]
mod raw_http;
