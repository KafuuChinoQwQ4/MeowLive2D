mod support;
use meowlive_server::transport::http::router;
use serde_json::json;
use support::*;

#[tokio::test]
async fn agent_is_paused_and_unconfigured_until_explicit_resume_is_possible() {
    let state = state();
    let (code, snapshot) = request(router(state.clone()), "GET", "/api/agent", json!({})).await;
    assert_eq!(code, 200);
    assert_eq!(snapshot["paused"], true);
    assert_eq!(snapshot["llm_configured"], false);
    assert_eq!(snapshot["bridge_connected"], false);
    assert!(snapshot.get("api_key_env").is_none());
    let (code, _) = request(router(state), "POST", "/api/agent/resume", json!({})).await;
    assert_eq!(code, 409);
}

#[tokio::test]
async fn event_batch_is_validated_atomically_and_replay_ids_are_deduplicated() {
    let state = state();
    let event = json!({"id":"e1","source":"simulator","viewer":"观众","kind":{"type":"chat","text":"你好"}});
    let (code, result) = request(
        router(state.clone()),
        "POST",
        "/api/events",
        json!({"events":[event.clone()]}),
    )
    .await;
    assert_eq!(code, 202);
    assert_eq!(result, json!({"accepted":1,"duplicates":0}));
    let (_, result) = request(
        router(state.clone()),
        "POST",
        "/api/events",
        json!({"events":[event.clone()]}),
    )
    .await;
    assert_eq!(result, json!({"accepted":0,"duplicates":1}));
    let mut second = event.clone();
    second["id"] = json!("e2");
    let mut invalid = event;
    invalid["id"] = json!("e3");
    invalid["kind"]["text"] = json!("");
    let (code, _) = request(
        router(state.clone()),
        "POST",
        "/api/events",
        json!({"events":[second,invalid]}),
    )
    .await;
    assert_eq!(code, 400);
    let (_, snapshot) = request(router(state), "GET", "/api/agent", json!({})).await;
    assert_eq!(snapshot["events"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn saving_settings_pauses_and_stop_clears_pending_events() {
    let state = state();
    let mut settings =
        json!({"persona":"温柔猫咪","topic":"游戏","proactive_enabled":true,"cooldown_ms":1000});
    let (code, snapshot) = request(
        router(state.clone()),
        "POST",
        "/api/agent/settings",
        settings.clone(),
    )
    .await;
    assert_eq!(code, 200);
    settings["interaction"] =
        serde_json::to_value(meowlive_protocol::agent::InteractionSettings::default()).unwrap();
    assert_eq!(snapshot["settings"], settings);
    assert_eq!(snapshot["paused"], true);
    let event = json!({"id":"gift-1","source":"simulator","viewer":"观众","kind":{"type":"gift","name":"花","count":2}});
    assert_eq!(
        request(
            router(state.clone()),
            "POST",
            "/api/events",
            json!({"events":[event]})
        )
        .await
        .0,
        202
    );
    assert_eq!(
        request(router(state.clone()), "POST", "/api/stop", json!({}))
            .await
            .0,
        200
    );
    let (_, snapshot) = request(router(state), "GET", "/api/agent", json!({})).await;
    assert_eq!(snapshot["paused"], true);
    assert_eq!(snapshot["events"][0]["status"], "cancelled");
}

#[tokio::test]
async fn rejects_empty_oversized_and_unknown_event_payloads() {
    let state = state();
    let event = json!({"id":"e1","source":"simulator","viewer":"viewer","kind":{"type":"chat","text":"hello"}});
    for payload in [
        json!({"events":[]}),
        json!({"events":vec![event.clone();101]}),
        json!({"events":[event],"key":"secret"}),
    ] {
        assert_eq!(
            request(router(state.clone()), "POST", "/api/events", payload)
                .await
                .0,
            400
        );
    }
}
