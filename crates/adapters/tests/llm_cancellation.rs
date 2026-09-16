mod llm_support;

use meowlive_adapters::llm::openai_compatible::OpenAiCompatible;
use meowlive_application::ports::llm::LanguageModel;
use std::{sync::Arc, time::Duration};
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn dropping_decision_future_closes_an_in_flight_request() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (received_tx, received_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(stream.read_u8().await.unwrap());
        }
        let headers = String::from_utf8(request).unwrap();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        let mut body = vec![0; content_length];
        stream.read_exact(&mut body).await.unwrap();
        received_tx.send(()).unwrap();
        let mut byte = [0; 1];
        tokio::time::timeout(Duration::from_secs(1), stream.read(&mut byte))
            .await
            .expect("cancelled request should close promptly")
            .expect("socket read should succeed")
    });

    let adapter = Arc::new(OpenAiCompatible::new(llm_support::config(url)).unwrap());
    let running = {
        let adapter = adapter.clone();
        tokio::spawn(async move { adapter.decide(llm_support::request(Vec::new())).await })
    };
    received_rx.await.unwrap();
    running.abort();
    assert_eq!(server.await.unwrap(), 0);
}
