use axum::{Json, Router, extract::Query, http::HeaderMap, routing::get};
use meowlive_adapters::search::{HttpWebSearch, SearchProvider, valid_endpoint};
use meowlive_application::ports::web_search::WebSearch;
use serde_json::json;
use std::{collections::HashMap, time::Duration};

#[tokio::test]
async fn native_search_formats_normalize_and_filter_unusable_sources() {
    async fn brave(
        headers: HeaderMap,
        Query(q): Query<HashMap<String, String>>,
    ) -> Json<serde_json::Value> {
        assert_eq!(headers["x-subscription-token"], "test-key");
        assert_eq!(q["q"], "新梗 含义");
        assert_eq!(q["count"], "5");
        Json(json!({"web":{"results":[
            {"title":"新梗资料", "url":"https://example.org/article", "description":"<b>公开解释</b>"},
            {"title":"local", "url":"http://127.0.0.1/secret", "description":"private"},
            {"title":"unsafe", "url":"javascript:alert(1)", "description":"bad"}
        ]}}))
    }
    async fn searx(Query(q): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
        assert_eq!(q["format"], "json");
        Json(
            json!({"results":[{"title":"资料", "url":"https://example.com/info", "content":"解释"}]}),
        )
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/brave", get(brave))
                .route("/search", get(searx)),
        )
        .await
        .unwrap();
    });
    let brave = HttpWebSearch::new(
        SearchProvider::Brave,
        &format!("{base}/brave"),
        Some("test-key".into()),
        Duration::from_secs(2),
    )
    .unwrap();
    let results = brave.search("新梗 含义".into()).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].snippet, "公开解释");
    let searx = HttpWebSearch::new(
        SearchProvider::Searxng,
        &format!("{base}/search"),
        None,
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(
        searx.search("资料".into()).await.unwrap()[0].snippet,
        "解释"
    );
    assert!(searx.search("\n".into()).await.is_err());
    assert!(searx.search("字".repeat(241)).await.is_err());
    server.abort();
}

#[tokio::test]
async fn errors_and_oversized_responses_do_not_expose_remote_bodies() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/search", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/search", get(|| async { "secret".repeat(60_000) })),
        )
        .await
        .unwrap();
    });
    let search = HttpWebSearch::new(
        SearchProvider::Searxng,
        &endpoint,
        None,
        Duration::from_secs(2),
    )
    .unwrap();
    let error = search.search("test".into()).await.unwrap_err();
    assert!(error.contains("过大"));
    assert!(!error.contains("secret"));
    server.abort();
}

#[test]
fn endpoints_are_explicit_and_do_not_embed_credentials() {
    assert!(valid_endpoint("https://search.example.com/search"));
    assert!(valid_endpoint("http://127.0.0.1:8080/search"));
    for url in [
        "http://example.com/search",
        "file:///secret",
        "https://key@example.com/search",
        "https://example.com/?token=key",
        "https://example.com/#secret",
    ] {
        assert!(!valid_endpoint(url), "{url}");
    }
}
