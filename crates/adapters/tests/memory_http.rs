use axum::{Json, Router, routing::post};
use meowlive_adapters::memory::{AdapterConfig, HttpMemoryAdapter};
use meowlive_application::ports::memory::{MemoryEmbedder, MemoryExtractor};
use meowlive_domain::memory::MemorySource;
use serde_json::{Value, json};
async fn adapter(body: Value) -> HttpMemoryAdapter {
    let app = Router::new()
        .route(
            "/chat/completions",
            post({
                let b = body.clone();
                move |Json(req): Json<Value>| {
                    let b = b.clone();
                    async move {
                        assert!(req.get("tools").is_none());
                        assert_eq!(req["response_format"]["type"], "json_object");
                        Json(b)
                    }
                }
            }),
        )
        .route(
            "/embeddings",
            post(move || {
                let b = body.clone();
                async move { Json(b) }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    HttpMemoryAdapter::new(AdapterConfig {
        endpoint,
        api_key: "private-secret".into(),
        model: "test".into(),
        dimensions: 2,
        timeout_ms: 1000,
    })
    .unwrap()
}
fn sources() -> Vec<MemorySource> {
    vec![MemorySource {
        source: "test".into(),
        viewer_id: "v".into(),
        event_id: "e".into(),
        text: "我喜欢猫".into(),
        occurred_at_ms: 1,
        day: 0,
    }]
}
#[tokio::test]
async fn rejects_forged_evidence() {
    let a=adapter(json!({"choices":[{"message":{"content":json!({"candidates":[{"key":"preference","value":"狗","kind":"preference","evidence":[{"source":"test","event_id":"e","quote":"我喜欢猫"}],"explicit":true,"confidence":1.0}]}).to_string()}}]})).await;
    assert!(a.extract(&sources()).await.is_err());
}
#[tokio::test]
async fn extracts_grounded_evidence_with_server_time() {
    let a=adapter(json!({"choices":[{"message":{"content":json!({"candidates":[{"key":"preference","value":"猫","kind":"preference","evidence":[{"source":"test","event_id":"e","quote":"我喜欢猫"}],"explicit":true,"confidence":1.0}]}).to_string()}}]})).await;
    assert_eq!(
        a.extract(&sources()).await.unwrap()[0].evidence[0].occurred_at_ms,
        1
    );
}
#[tokio::test]
async fn rejects_wrong_dimensions_and_redacts_errors() {
    let a = adapter(
        json!({"model":"test","data":[{"index":0,"embedding":[1.0]}],"private-secret":"secret"}),
    )
    .await;
    let err = a.embed(&["猫".into()]).await.unwrap_err();
    assert!(!err.message.contains("secret"));
}
#[tokio::test]
async fn embeddings_are_reordered_by_index() {
    let a=adapter(json!({"model":"test","data":[{"index":1,"embedding":[2.0,3.0]},{"index":0,"embedding":[0.0,1.0]}]})).await;
    assert_eq!(
        a.embed(&["a".into(), "b".into()]).await.unwrap().vectors[0],
        vec![0.0, 1.0]
    );
}
#[tokio::test]
async fn oversized_response_is_rejected() {
    let a = adapter(json!({"error":"x".repeat(65537)})).await;
    assert!(a.embed(&["a".into()]).await.is_err());
}
#[tokio::test]
async fn non_finite_f32_and_duplicate_indices_are_rejected() {
    for body in [
        json!({"model":"test","data":[{"index":0,"embedding":[1e100,0.0]}]}),
        json!({"model":"test","data":[{"index":0,"embedding":[1.0,0.0]},{"index":0,"embedding":[1.0,0.0]}]}),
    ] {
        let count = body["data"].as_array().unwrap().len();
        let a = adapter(body).await;
        assert!(a.embed(&vec!["x".into(); count]).await.is_err());
    }
}
#[tokio::test]
async fn rejects_unknown_fields_and_forged_source() {
    for evidence in [
        json!({"source":"forged","event_id":"e","quote":"我喜欢猫"}),
        json!({"source":"test","event_id":"e","quote":"我喜欢猫","day":999}),
    ] {
        let a=adapter(json!({"choices":[{"message":{"content":json!({"candidates":[{"key":"preference","value":"猫","kind":"preference","evidence":[evidence],"explicit":true,"confidence":1.0}]}).to_string()}}]})).await;
        assert!(a.extract(&sources()).await.is_err());
    }
}
#[tokio::test]
async fn upstream_error_body_never_leaks() {
    let app = Router::new().route(
        "/embeddings",
        post(|| async {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "private-secret raw prompt",
            )
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let a = HttpMemoryAdapter::new(AdapterConfig {
        endpoint,
        api_key: "private-secret".into(),
        model: "test".into(),
        dimensions: 2,
        timeout_ms: 1000,
    })
    .unwrap();
    let err = a.embed(&["a".into()]).await.unwrap_err();
    assert!(err.retryable);
    assert_eq!(err.message, "memory service rejected request");
}
