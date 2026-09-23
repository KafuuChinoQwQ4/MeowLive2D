mod agent_support;
mod support;
use agent_support::*;
use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{get, post},
};
use meowlive_adapters::llm::multi_provider::{ApiFormat, LlmConfig, MultiProvider};
use meowlive_desktop_runtime::{
    audio::SimulatedBackend, config::ClientConfig, connection::run_once,
};
use meowlive_protocol::{
    agent::{AgentEventStatus, EventBatchRequest},
    agent_observability::{AgentTraceStatus, AgentTraceStepKind},
    llm_runtime::{AgentRuntimeSettingsRequest, LlmPrice},
};
use meowlive_server::{config::AppConfig, llm_runtime::UsageQuery};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[tokio::test]
async fn native_stream_search_followup_playback_receipt_and_usage_form_one_flow() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let searched = Arc::new(AtomicUsize::new(0));
    let search_count = searched.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let model_route = post(move |Json(body): Json<Value>| {
        let index = counted.fetch_add(1, Ordering::SeqCst);
        async move {
            assert_eq!(body["stream"], true);
            let mut frames = vec![];
            if index == 0 {
                assert!(
                    body["tools"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|tool| tool["function"]["name"] == "web_search")
                );
                frames.push(json!({"choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"lookup-1","type":"function","function":{"name":"web_search","arguments":"{\"query\":\"新梗 含义\"}"}}]},"finish_reason":null}]}));
                frames
                    .push(json!({"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}));
            } else {
                assert_eq!(index, 1);
                let messages = body["messages"].as_array().unwrap();
                let result = messages
                    .iter()
                    .find(|message| message["role"] == "tool")
                    .unwrap();
                assert_eq!(result["tool_call_id"], "lookup-1");
                assert!(result["content"].as_str().unwrap().contains("公开词条"));
                frames.push(json!({"choices":[{"index":0,"delta":{"content":json!({"reply_to":["event-search"],"text":"刚查到这个词的公开解释了。","topic":"网络用语"}).to_string()},"finish_reason":null}]}));
                frames.push(json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}));
            }
            frames.push(json!({"choices":[],"usage":{"prompt_tokens":100,"completion_tokens":20,"prompt_tokens_details":{"cached_tokens":80},"completion_tokens_details":{"reasoning_tokens":5}}}));
            let mut stream = frames
                .iter()
                .map(|value| format!("data: {value}\n\n"))
                .collect::<String>();
            stream.push_str("data: [DONE]\n\n");
            ([("content-type", "text/event-stream")], stream).into_response()
        }
    });
    let upstream = tokio::spawn(async move {
        axum::serve(listener,Router::new().route("/v1/chat/completions",model_route).route("/search",get(move || {search_count.fetch_add(1,Ordering::SeqCst);async{Json(json!({"results":[{"title":"公开词条","url":"https://example.com/entry","content":"公开词条中的解释"}]}))}}))).await.unwrap();
    });
    let mut config = AppConfig::default();
    config.agent.cooldown_ms = 1000;
    config.llm.base_url = format!("{base}/v1");
    config.llm.model = "integration-model".into();
    config.llm.provider = "custom".into();
    let model = MultiProvider::new(
        LlmConfig {
            base_url: config.llm.base_url.clone(),
            model: config.llm.model.clone(),
            api_key: None,
            timeout: Duration::from_secs(3),
            max_response_bytes: 65536,
            max_tokens: 256,
            json_mode: true,
        },
        ApiFormat::OpenaiChat,
    )
    .unwrap();
    let harness = Harness::configured(config, Arc::new(model)).await;
    let mut settings = harness.state.llm_runtime.settings();
    settings.web_search_enabled = true;
    settings.search_provider = "searxng".into();
    settings.search_endpoint = format!("{base}/search");
    settings.prices.push(LlmPrice {
        provider: "custom".into(),
        base_url: format!("{base}/v1"),
        model: "integration-model".into(),
        input_usd_per_million: 2.0,
        output_usd_per_million: 10.0,
        cache_read_usd_per_million: 0.5,
        cache_write_usd_per_million: 2.0,
    });
    harness
        .state
        .llm_runtime
        .save(AgentRuntimeSettingsRequest {
            settings,
            search_api_key: None,
            clear_search_api_key: false,
        })
        .unwrap();
    let desktop_config = ClientConfig::from_toml(&format!(
        "server_url='{}'",
        harness.base.replace("ws://", "http://")
    ))
    .unwrap();
    let desktop = tokio::spawn(async move {
        run_once(
            &desktop_config,
            SimulatedBackend::new(desktop_config.max_buffer_samples),
        )
        .await
    });
    support::await_connected(&harness.state, true).await;
    harness
        .state
        .submit_events(EventBatchRequest {
            events: vec![event("event-search")],
        })
        .await
        .unwrap();
    harness.state.resume_agent().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if harness.state.agent_snapshot().await.events[0].status == AgentEventStatus::Completed
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(searched.load(Ordering::SeqCst), 1);
    assert_eq!(harness.synthesis_calls.load(Ordering::SeqCst), 1);
    assert!(
        harness.state.snapshot().await.speeches[0]
            .text
            .contains("刚查到")
    );
    let activity = harness.state.llm_runtime.activity();
    assert_eq!(activity.phase, "completed");
    assert_eq!(activity.tools[0].sources, vec!["https://example.com/entry"]);
    let ledger = harness.state.llm_runtime.usage(&UsageQuery::default());
    assert_eq!(ledger.totals.calls, 2);
    assert_eq!(ledger.totals.input_tokens, 200);
    assert_eq!(ledger.totals.cache_read_tokens, 160);
    assert_eq!(ledger.totals.output_tokens, 40);
    assert_eq!(ledger.totals.reasoning_tokens, 10);
    assert_eq!(ledger.totals.estimated_cost_microusd, 560);
    assert!(
        ledger
            .records
            .iter()
            .all(|record| record.first_token_ms.is_some() && record.status == "completed")
    );
    let traces = harness.state.agent_observability.list(10, None);
    assert_eq!(traces.traces.len(), 1);
    let trace = harness
        .state
        .agent_observability
        .get(&traces.traces[0].id)
        .unwrap();
    assert_eq!(trace.summary.status, AgentTraceStatus::Completed);
    for kind in [
        AgentTraceStepKind::SpeechSynthesizing,
        AgentTraceStepKind::SpeechReady,
    ] {
        assert!(trace.steps.iter().any(|step| step.kind == kind));
    }
    assert_eq!(trace.turns.len(), 2);
    assert_eq!(trace.summary.tool_count, 1);
    assert!(trace.steps.iter().any(|step| {
        step.kind == AgentTraceStepKind::ToolFinished
            && step.turn_id.as_deref() == Some(trace.turns[0].id.as_str())
            && step.sources == vec!["https://example.com/entry"]
    }));
    assert!(
        trace
            .steps
            .iter()
            .any(|step| step.kind == AgentTraceStepKind::SpeechPlaying)
    );
    assert!(ledger.records.iter().all(|record| {
        record.trace_id.as_deref() == Some(trace.summary.id.as_str()) && record.turn_id.is_some()
    }));
    harness.state.shutdown().await;
    assert!(desktop.await.unwrap().is_err());
    upstream.abort();
}
