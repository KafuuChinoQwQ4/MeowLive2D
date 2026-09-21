mod llm_support;

use std::{collections::HashMap, time::Duration};

use axum::{Json, Router, extract::Query, http::HeaderMap, routing::get};
use meowlive_adapters::llm::{
    models::{AvailableModel, ModelCatalogConfig, canonical_base_url, list_models},
    multi_provider::ApiFormat,
};
use serde_json::json;

fn config(base_url: String, api_format: ApiFormat) -> ModelCatalogConfig {
    ModelCatalogConfig {
        base_url,
        api_key: Some("catalog-secret".into()),
        api_format,
        timeout: Duration::from_secs(2),
    }
}

#[test]
fn canonical_bases_match_inference_endpoints_but_keep_distinct_prefixes_separate() {
    for (input, expected) in [
        ("https://example.test", "https://example.test"),
        ("https://example.test/v1", "https://example.test/v1"),
        (
            "https://example.test/v1/chat/completions",
            "https://example.test/v1",
        ),
        (
            "https://example.test/chat/completions",
            "https://example.test",
        ),
        ("https://example.test/models", "https://example.test"),
    ] {
        assert_eq!(
            canonical_base_url(input, ApiFormat::OpenaiChat).unwrap(),
            expected
        );
    }
    assert_eq!(
        canonical_base_url("https://example.test", ApiFormat::GeminiGenerateContent).unwrap(),
        "https://example.test"
    );
    assert_eq!(
        canonical_base_url(
            "https://example.test/custom/v1/models/old:generateContent",
            ApiFormat::GeminiGenerateContent
        )
        .unwrap(),
        "https://example.test/custom/v1"
    );
    assert_ne!(
        canonical_base_url("https://example.test/vendor-a", ApiFormat::OpenaiChat).unwrap(),
        canonical_base_url("https://example.test/vendor-b", ApiFormat::OpenaiChat).unwrap()
    );
    assert_ne!(
        canonical_base_url("https://example.test", ApiFormat::OpenaiChat).unwrap(),
        canonical_base_url("https://example.test/v1", ApiFormat::OpenaiChat).unwrap()
    );
}

#[tokio::test]
async fn openai_lists_actual_ids_with_bearer_auth_and_sorts_without_model_name_filtering() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1/models",
        get(|headers: HeaderMap| async move {
            assert_eq!(headers["authorization"], "Bearer catalog-secret");
            assert_eq!(headers["accept"], "application/json");
            assert!(!headers.contains_key("x-api-key"));
            Json(json!({"object": "list", "data": [
                {"id": "vendor/unknown-model", "object": "model"},
                {"id": "embedding-name-is-not-a-reliable-filter"},
                {"id": "vendor/unknown-model"}
            ]}))
        }),
    ))
    .await;
    for api_format in [ApiFormat::OpenaiChat, ApiFormat::OpenaiResponses] {
        let catalog = list_models(config(url.clone(), api_format)).await.unwrap();
        assert_eq!(catalog.base_url, format!("{url}/v1"));
        assert_eq!(
            catalog.models,
            vec![
                AvailableModel {
                    id: "embedding-name-is-not-a-reliable-filter".into(),
                    name: "embedding-name-is-not-a-reliable-filter".into()
                },
                AvailableModel {
                    id: "vendor/unknown-model".into(),
                    name: "vendor/unknown-model".into()
                },
            ]
        );
    }
    task.abort();
}

#[tokio::test]
async fn supplied_base_prefixes_and_complete_inference_urls_resolve_to_the_same_catalog() {
    let (url, task) = llm_support::server(Router::new().route(
        "/gateway/v1/models",
        get(|| async { Json(json!({"data": [{"id": "model-a"}]})) }),
    ))
    .await;
    for suffix in [
        "/gateway/v1",
        "/gateway/v1/",
        "/gateway/v1/models",
        "/gateway/v1/chat/completions",
        "/gateway/v1/responses",
        "/gateway/v1/messages",
        "/gateway/v1/models/previous-model:generateContent",
    ] {
        let catalog = list_models(config(format!("{url}{suffix}"), ApiFormat::OpenaiChat))
            .await
            .unwrap();
        assert_eq!(catalog.base_url, format!("{url}/gateway/v1"));
        assert_eq!(catalog.models[0].id, "model-a");
    }
    task.abort();
}

#[tokio::test]
async fn root_openai_and_anthropic_urls_can_fall_back_to_unversioned_models() {
    let (url, task) = llm_support::server(Router::new().route(
        "/models",
        get(|| async { Json(json!({"data": [{"id": "local-model"}], "has_more": false})) }),
    ))
    .await;
    for api_format in [ApiFormat::OpenaiChat, ApiFormat::AnthropicMessages] {
        let catalog = list_models(config(url.clone(), api_format)).await.unwrap();
        assert_eq!(catalog.base_url, url);
        assert_eq!(catalog.models[0].id, "local-model");
    }
    task.abort();
}

#[tokio::test]
async fn anthropic_follows_after_id_and_uses_native_authentication() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1/models",
        get(|headers: HeaderMap, Query(query): Query<HashMap<String, String>>| async move {
            assert_eq!(headers["x-api-key"], "catalog-secret");
            assert_eq!(headers["anthropic-version"], "2023-06-01");
            assert!(!headers.contains_key("authorization"));
            if let Some(after) = query.get("after_id") {
                assert_eq!(after, "claude-z");
                Json(json!({"data": [{"id": "claude-a", "display_name": "Claude A"}], "has_more": false, "first_id": "claude-a", "last_id": "claude-a"}))
            } else {
                Json(json!({"data": [{"id": "claude-z", "display_name": "Claude Z"}], "has_more": true, "first_id": "claude-z", "last_id": "claude-z"}))
            }
        }),
    ))
    .await;
    let catalog = list_models(config(url.clone(), ApiFormat::AnthropicMessages))
        .await
        .unwrap();
    assert_eq!(catalog.base_url, format!("{url}/v1"));
    assert_eq!(
        catalog.models,
        vec![
            AvailableModel {
                id: "claude-a".into(),
                name: "Claude A".into()
            },
            AvailableModel {
                id: "claude-z".into(),
                name: "Claude Z".into()
            },
        ]
    );
    task.abort();
}

#[tokio::test]
async fn gemini_follows_page_tokens_and_lists_only_generate_content_models() {
    let (url, task) = llm_support::server(Router::new().route(
        "/v1beta/models",
        get(|headers: HeaderMap, Query(query): Query<HashMap<String, String>>| async move {
            assert_eq!(headers["x-goog-api-key"], "catalog-secret");
            assert!(!headers.contains_key("authorization"));
            assert!(!query.contains_key("key"));
            if let Some(token) = query.get("pageToken") {
                assert_eq!(token, "page+two/=&");
                Json(json!({"models": [
                    {"name": "models/gemini-b", "displayName": "Gemini B", "supportedGenerationMethods": ["generateContent"]}
                ]}))
            } else {
                Json(json!({"models": [
                    {"name": "models/embedding", "displayName": "Embedding", "supportedGenerationMethods": ["embedContent"]},
                    {"name": "models/gemini-a", "displayName": "Gemini A", "supportedGenerationMethods": ["countTokens", "generateContent"]}
                ], "nextPageToken": "page+two/=&"}))
            }
        }),
    ))
    .await;
    let catalog = list_models(config(url.clone(), ApiFormat::GeminiGenerateContent))
        .await
        .unwrap();
    assert_eq!(catalog.base_url, format!("{url}/v1beta"));
    assert_eq!(
        catalog.models,
        vec![
            AvailableModel {
                id: "gemini-a".into(),
                name: "Gemini A".into()
            },
            AvailableModel {
                id: "gemini-b".into(),
                name: "Gemini B".into()
            },
        ]
    );
    task.abort();
}

#[tokio::test]
async fn empty_catalog_is_distinguished_from_an_unsupported_response_shape() {
    for (body, successful) in [
        (json!({"data": []}), true),
        (json!({}), false),
        (json!({"data": null}), false),
        (json!({"data": {"id": "a"}}), false),
        (json!({"data": [{"id": 3}]}), false),
        (json!({"data": [null]}), false),
    ] {
        let (url, task) = llm_support::server(
            Router::new().route("/v1/models", get(move || async move { Json(body) })),
        )
        .await;
        let result = list_models(config(url, ApiFormat::OpenaiChat)).await;
        assert_eq!(result.is_ok(), successful, "{result:?}");
        if let Ok(catalog) = result {
            assert!(catalog.models.is_empty());
        }
        task.abort();
    }
}
