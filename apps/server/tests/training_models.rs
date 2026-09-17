mod support;
use meowlive_server::transport::http::router;
use serde_json::json;

#[tokio::test]
async fn model_switch_reports_unsupported_for_legacy_engine() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut state = support::state();
    std::sync::Arc::make_mut(&mut state.config).speech.base_url =
        format!("http://{}", listener.local_addr().unwrap());
    let server =
        tokio::spawn(async move { axum::serve(listener, axum::Router::new()).await.unwrap() });
    let (code, value) = support::request(
        router(state.clone()),
        "GET",
        "/api/training/models",
        json!({}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(value["supported"], false);
    assert_eq!(value["state"], "unsupported");
    let (code, _) = support::request(
        router(state.clone()),
        "POST",
        "/api/training/models",
        json!({"enabled":true}),
    )
    .await;
    assert_eq!(code, 409);
    let (code, _) = support::request(
        router(state),
        "POST",
        "/api/training/models",
        json!({"enabled":"yes"}),
    )
    .await;
    assert_eq!(code, 400);
    server.abort();
}

#[tokio::test]
async fn project_model_switch_works_without_enabling_training_or_weight_management() {
    use axum::{Json, Router, extract::State, routing::get};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    async fn status(State(loaded): State<Arc<AtomicBool>>) -> Json<serde_json::Value> {
        Json(
            json!({"supported":true,"state":if loaded.load(Ordering::SeqCst) {"loaded"} else {"unloaded"},"message":"controlled"}),
        )
    }
    async fn control(
        State(loaded): State<Arc<AtomicBool>>,
        Json(body): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        loaded.store(body["enabled"].as_bool().unwrap(), Ordering::SeqCst);
        status(State(loaded)).await
    }
    let loaded = Arc::new(AtomicBool::new(false));
    let engine = Router::new()
        .route("/meowlive/models", get(status).post(control))
        .with_state(loaded.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut state = support::state();
    Arc::make_mut(&mut state.config).speech.base_url =
        format!("http://{}", listener.local_addr().unwrap());
    assert!(!state.config.training.enabled);
    assert!(!state.config.training.managed_inference);
    let server = tokio::spawn(async move { axum::serve(listener, engine).await.unwrap() });
    let (code, value) = support::request(
        router(state.clone()),
        "GET",
        "/api/training/models",
        json!({}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(value["state"], "unloaded");
    for enabled in [true, false] {
        let (code, value) = support::request(
            router(state.clone()),
            "POST",
            "/api/training/models",
            json!({"enabled":enabled}),
        )
        .await;
        assert_eq!(code, 200, "{value}");
        assert_eq!(loaded.load(Ordering::SeqCst), enabled);
    }
    assert!(state.training.snapshot().jobs.is_empty());
    server.abort();
}
