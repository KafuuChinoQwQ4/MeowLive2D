mod llm_support;

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    body::Body,
    extract::Query,
    http::{Response, StatusCode, header},
    routing::get,
};
use meowlive_adapters::llm::{
    models::{ModelCatalogConfig, list_models},
    multi_provider::ApiFormat,
};
use serde_json::{Value, json};

fn config(base_url: String, api_format: ApiFormat) -> ModelCatalogConfig {
    ModelCatalogConfig {
        base_url,
        api_key: Some("private-catalog-key".into()),
        api_format,
        timeout: Duration::from_secs(2),
    }
}

#[tokio::test]
async fn invalid_addresses_and_keys_are_rejected_before_any_http_request() {
    let hits = Arc::new(AtomicUsize::new(0));
    let received = hits.clone();
    let (url, task) = llm_support::server(Router::new().fallback(get(move || {
        received.fetch_add(1, Ordering::SeqCst);
        async { Json(json!({"data": []})) }
    })))
    .await;
    let authority = url.strip_prefix("http://").unwrap();
    for bad_url in [
        format!("{url}/v1?key=private-catalog-key"),
        format!("{url}/v1#secret"),
        format!("http://user:pass@{authority}/v1"),
        format!("http://@{authority}/v1"),
        format!("http:@{authority}/v1"),
        format!("{url}/v1\t"),
        format!("{url}/{}", "a".repeat(4096)),
        format!("ftp://{authority}/v1"),
        format!("{url}/v1\u{0001}"),
    ] {
        assert!(
            list_models(config(bad_url.clone(), ApiFormat::OpenaiChat))
                .await
                .is_err(),
            "accepted {bad_url:?}"
        );
    }
    for key in [
        String::new(),
        " ".into(),
        "secret\r\nInjected: yes".into(),
        "a".repeat(4097),
    ] {
        let mut settings = config(url.clone(), ApiFormat::OpenaiChat);
        settings.api_key = Some(key);
        assert!(list_models(settings).await.is_err());
    }
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    task.abort();
}

#[tokio::test]
async fn unsupported_openai_pagination_is_an_error_instead_of_a_partial_catalog() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1/models",
        get(|| async {
            Json(json!({"data": [{"id": "first-model"}], "has_more": true, "last_id": "first-model"}))
        }),
    ))
    .await;
    assert!(
        list_models(config(url, ApiFormat::OpenaiChat))
            .await
            .is_err()
    );
    task.abort();
}

#[tokio::test]
async fn invalid_time_budgets_do_not_start_requests() {
    let hits = Arc::new(AtomicUsize::new(0));
    let received = hits.clone();
    let (url, task) = llm_support::server(Router::new().route(
        "/v1/models",
        get(move || {
            received.fetch_add(1, Ordering::SeqCst);
            async { Json(json!({"data": []})) }
        }),
    ))
    .await;
    for timeout in [Duration::ZERO, Duration::from_secs(31)] {
        let mut settings = config(url.clone(), ApiFormat::OpenaiChat);
        settings.timeout = timeout;
        assert!(list_models(settings).await.is_err());
    }
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    task.abort();
}

#[tokio::test]
async fn the_timeout_budget_covers_all_pages_instead_of_restarting_per_page() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1beta/models",
        get(|Query(query): Query<HashMap<String, String>>| async move {
            tokio::time::sleep(Duration::from_millis(70)).await;
            match query.get("pageToken").map(String::as_str) {
                None => Json(json!({"models": [], "nextPageToken": "second"})),
                Some("second") => Json(json!({"models": [], "nextPageToken": "third"})),
                _ => Json(json!({"models": []})),
            }
        }),
    ))
    .await;
    let mut settings = config(url, ApiFormat::GeminiGenerateContent);
    settings.timeout = Duration::from_millis(150);
    let result = list_models(settings).await;
    assert!(
        result.is_err(),
        "paginated response outlived the overall budget"
    );
    assert!(result.unwrap_err().retryable);
    task.abort();
}

#[tokio::test]
async fn oversized_chunked_response_is_rejected_without_relying_on_content_length() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1/models",
        get(|| async {
            let bytes = json!({"data": [], "padding": "x".repeat(1024 * 1024)}).to_string();
            let chunks: Vec<Result<Vec<u8>, std::io::Error>> = bytes
                .as_bytes()
                .chunks(16384)
                .map(|chunk| Ok(chunk.to_vec()))
                .collect();
            Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from_stream(futures_util::stream::iter(chunks)))
                .unwrap()
        }),
    ))
    .await;
    assert!(
        list_models(config(url, ApiFormat::OpenaiChat))
            .await
            .is_err()
    );
    task.abort();
}

#[tokio::test]
async fn response_byte_limit_is_shared_across_pages() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1beta/models",
        get(|Query(query): Query<HashMap<String, String>>| async move {
            let mut body = json!({"models": [], "padding": "x".repeat(600 * 1024)});
            if !query.contains_key("pageToken") {
                body["nextPageToken"] = json!("second");
            }
            Json(body)
        }),
    ))
    .await;
    assert!(
        list_models(config(url, ApiFormat::GeminiGenerateContent))
            .await
            .is_err()
    );
    task.abort();
}

#[tokio::test]
async fn model_entry_limit_counts_duplicates_and_filtered_entries_too() {
    for (format, body) in [
        (
            ApiFormat::OpenaiChat,
            json!({"data": vec![json!({"id": "duplicate"}); 1001]}),
        ),
        (
            ApiFormat::GeminiGenerateContent,
            json!({"models": vec![json!({"name": "models/embedding", "supportedGenerationMethods": ["embedContent"]}); 1001]}),
        ),
    ] {
        let (url, task) =
            llm_support::server(Router::new().fallback(get(move || async move { Json(body) })))
                .await;
        assert!(list_models(config(url, format)).await.is_err());
        task.abort();
    }
}

#[tokio::test]
async fn scanning_limit_is_cumulative_across_pages() {
    let (url, task) = llm_support::server(Router::new().route("/v1/models", get(
        |Query(query): Query<HashMap<String, String>>| async move {
            Json(json!({"data": vec![json!({"id": "duplicate"}); 600], "has_more": !query.contains_key("after_id"), "last_id": "duplicate"}))
        }
    ))).await;
    assert!(
        list_models(config(url, ApiFormat::AnthropicMessages))
            .await
            .is_err()
    );
    task.abort();
}

#[tokio::test]
async fn pagination_stops_with_an_error_before_requesting_an_eleventh_page() {
    let hits = Arc::new(AtomicUsize::new(0));
    let received = hits.clone();
    let (url, task) = llm_support::server(Router::new().route(
        "/v1beta/models",
        get(move || {
            let page = received.fetch_add(1, Ordering::SeqCst) + 1;
            async move {
                if page <= 10 {
                    Json(json!({"models": [], "nextPageToken": format!("page-{page}")}))
                } else {
                    Json(json!({"models": []}))
                }
            }
        }),
    ))
    .await;
    assert!(
        list_models(config(url, ApiFormat::GeminiGenerateContent))
            .await
            .is_err()
    );
    assert_eq!(hits.load(Ordering::SeqCst), 10);
    task.abort();
}

#[tokio::test]
async fn repeated_page_tokens_are_reported_instead_of_retried() {
    let hits = Arc::new(AtomicUsize::new(0));
    let received = hits.clone();
    let (url, task) = llm_support::server(Router::new().route(
        "/v1/models",
        get(move || {
            let page = received.fetch_add(1, Ordering::SeqCst);
            async move { Json(json!({"data": [], "has_more": page < 2, "last_id": "repeated"})) }
        }),
    ))
    .await;
    assert!(
        list_models(config(url, ApiFormat::AnthropicMessages))
            .await
            .is_err()
    );
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    task.abort();
}

#[tokio::test]
async fn malformed_ids_methods_and_pagination_metadata_fail_explicitly() {
    let mut cases: Vec<(ApiFormat, Value)> = ["", "two words", " leading", "line\nbreak"]
        .into_iter()
        .map(|id| (ApiFormat::OpenaiChat, json!({"data": [{"id": id}]})))
        .collect();
    cases.extend([
        (ApiFormat::OpenaiChat, json!({"data": [{"id": "x".repeat(129)}]})),
        (ApiFormat::OpenaiChat, json!({"data": [{"id": "ok", "display_name": 12}]})),
        (ApiFormat::AnthropicMessages, json!({"data": [], "has_more": "false"})),
        (ApiFormat::AnthropicMessages, json!({"data": [], "has_more": true, "last_id": ""})),
        (ApiFormat::GeminiGenerateContent, json!({"models": [], "nextPageToken": 3})),
        (ApiFormat::GeminiGenerateContent, json!({"models": [], "nextPageToken": "x".repeat(4097)})),
        (ApiFormat::GeminiGenerateContent, json!({"models": [{"name": "models/model-a", "supportedGenerationMethods": [3, "generateContent"]}]})),
    ]);
    for (format, body) in cases {
        let response = body.clone();
        let (url, task) = llm_support::server(Router::new().fallback(get(
            move |Query(query): Query<HashMap<String, String>>| {
                let response = response.clone();
                async move {
                    if query.is_empty() {
                        Json(response)
                    } else {
                        Json(json!({"data": [], "models": [], "has_more": false}))
                    }
                }
            },
        )))
        .await;
        assert!(
            list_models(config(url, format)).await.is_err(),
            "accepted {body}"
        );
        task.abort();
    }
}

#[tokio::test]
async fn unsafe_or_unusable_display_names_fall_back_to_the_valid_model_id() {
    for name in [
        String::new(),
        " ".into(),
        "x".repeat(257),
        "line\nbreak".into(),
    ] {
        let (url, task) = llm_support::server(Router::new().route(
            "/v1/models",
            get(move || async move {
                Json(json!({"data": [{"id": "usable-model", "display_name": name}]}))
            }),
        ))
        .await;
        let catalog = list_models(config(url, ApiFormat::OpenaiChat))
            .await
            .unwrap();
        assert_eq!(catalog.models[0].name, "usable-model");
        task.abort();
    }
}

#[tokio::test]
async fn authentication_errors_do_not_fall_back_or_expose_provider_response_text() {
    let hits = Arc::new(AtomicUsize::new(0));
    let received = hits.clone();
    let (url, task) = llm_support::server(
        Router::new()
            .route(
                "/v1/models",
                get(|| async {
                    (
                        StatusCode::UNAUTHORIZED,
                        "private-catalog-key upstream-secret",
                    )
                }),
            )
            .route(
                "/models",
                get(move || {
                    received.fetch_add(1, Ordering::SeqCst);
                    async { Json(json!({"data": []})) }
                }),
            ),
    )
    .await;
    let error = list_models(config(url, ApiFormat::OpenaiChat))
        .await
        .unwrap_err();
    assert!(error.message.contains("401"));
    assert!(!error.message.contains("private-catalog-key"));
    assert!(!error.message.contains("upstream-secret"));
    assert!(!error.retryable);
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    task.abort();
}

#[tokio::test]
async fn redirects_never_forward_the_api_key_to_their_target() {
    let hits = Arc::new(AtomicUsize::new(0));
    let received = hits.clone();
    let (target, target_task) = llm_support::server(Router::new().fallback(get(move || {
        received.fetch_add(1, Ordering::SeqCst);
        async { Json(json!({"data": []})) }
    })))
    .await;
    let (url, task) = llm_support::server(Router::new().route(
        "/v1/models",
        get(move || async move { (StatusCode::TEMPORARY_REDIRECT, [(header::LOCATION, target)]) }),
    ))
    .await;
    let error = list_models(config(url, ApiFormat::OpenaiChat))
        .await
        .unwrap_err();
    assert!(error.message.contains("307"));
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    task.abort();
    target_task.abort();
}

#[tokio::test]
async fn explicit_prefixes_do_not_fall_back_when_their_models_route_is_missing() {
    let hits = Arc::new(AtomicUsize::new(0));
    let received = hits.clone();
    let (url, task) = llm_support::server(Router::new().route(
        "/models",
        get(move || {
            received.fetch_add(1, Ordering::SeqCst);
            async { Json(json!({"data": []})) }
        }),
    ))
    .await;
    assert!(
        list_models(config(format!("{url}/v1"), ApiFormat::OpenaiChat))
            .await
            .is_err()
    );
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    task.abort();
}
