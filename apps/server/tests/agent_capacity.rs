mod support;
use meowlive_server::transport::http::router;
use serde_json::json;
use support::*;

fn events(start: usize, count: usize) -> serde_json::Value {
    json!({"events":(start..start+count).map(|i|json!({"id":format!("e{i}"),"source":"simulator","viewer":"观众","kind":{"type":"chat","text":"a".repeat(500)}})).collect::<Vec<_>>()})
}

#[tokio::test]
async fn event_route_accepts_valid_batches_above_speech_limit_and_rejects_capacity_atomically() {
    let state = state();
    let (code, result) =
        request(router(state.clone()), "POST", "/api/events", events(0, 100)).await;
    assert_eq!(code, 202);
    assert_eq!(result["accepted"], 100);
    let (code, result) = request(
        router(state.clone()),
        "POST",
        "/api/events",
        events(100, 29),
    )
    .await;
    assert_eq!(code, 429);
    assert_eq!(result["code"], "event_queue_full");
    let (_, snapshot) = request(router(state.clone()), "GET", "/api/agent", json!({})).await;
    assert_eq!(snapshot["events"].as_array().unwrap().len(), 100);
    assert_eq!(
        request(router(state), "POST", "/api/events", events(100, 28))
            .await
            .0,
        202
    );
}
