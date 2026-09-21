mod support;
use meowlive_application::ports::{
    llm::{AgentDecision, DecisionFuture, DecisionRequest, LanguageModel, LlmError},
    llm_runtime::{
        ModelEvent, ModelObserver, ModelOptions, ModelTurn, ModelTurnFuture, TokenUsage,
    },
};
use meowlive_server::llm_runtime::{UsageQuery, measured_turn};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
enum Ending {
    Success,
    Failure,
    Pending,
}
struct Model {
    ending: Ending,
    entered: Arc<tokio::sync::Notify>,
}
impl LanguageModel for Model {
    fn decide(&self, _: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async { unreachable!() })
    }
    fn turn(&self, _: DecisionRequest, options: ModelOptions) -> ModelTurnFuture<'_> {
        Box::pin(async move {
            let mut usage = TokenUsage {
                input_tokens: Some(10),
                output_tokens: Some(1),
                cache_read_tokens: Some(0),
                cache_write_tokens: Some(0),
                reasoning_tokens: Some(0),
            };
            if let Some(observer) = options.observer {
                observer.on_event(ModelEvent::FirstToken);
                observer.on_event(ModelEvent::Usage(usage.clone()));
                usage.output_tokens = Some(2);
                observer.on_event(ModelEvent::Usage(usage.clone()));
            }
            self.entered.notify_one();
            match self.ending {
                Ending::Failure => Err(LlmError::new(
                    "fixture upstream error must not reach ledger",
                    true,
                )),
                Ending::Pending => std::future::pending().await,
                Ending::Success => Ok(ModelTurn {
                    decision: Some(AgentDecision {
                        reply_to: vec![],
                        text: Some("test".into()),
                        topic: None,
                    }),
                    tool_calls: vec![],
                    continuation: None,
                    usage,
                    finish_reason: "stop".into(),
                }),
            }
        })
    }
}
#[derive(Default)]
struct Observer(Mutex<Vec<ModelEvent>>);
impl ModelObserver for Observer {
    fn on_event(&self, event: ModelEvent) {
        self.0.lock().unwrap().push(event);
    }
}
fn request() -> DecisionRequest {
    DecisionRequest {
        persona: "private-prompt-marker".into(),
        topic: String::new(),
        events: vec![],
        history: vec![],
        memory_context: vec![],
    }
}

#[tokio::test]
async fn snapshots_are_not_added_twice_and_upstream_observer_still_receives_events() {
    let state = support::state();
    let observer = Arc::new(Observer::default());
    let model = Model {
        ending: Ending::Success,
        entered: Arc::new(tokio::sync::Notify::new()),
    };
    measured_turn(
        &state,
        &model,
        request(),
        ModelOptions {
            observer: Some(observer.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let snapshot = state.llm_runtime.usage(&UsageQuery::default());
    assert_eq!(snapshot.totals.calls, 1);
    assert_eq!(snapshot.totals.input_tokens, 10);
    assert_eq!(snapshot.totals.output_tokens, 2);
    assert_eq!(snapshot.records[0].status, "completed");
    assert!(snapshot.records[0].first_token_ms.is_some());
    assert_eq!(snapshot.records[0].estimated_cost_microusd, None);
    assert_eq!(observer.0.lock().unwrap().len(), 3);
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("private-prompt-marker")
    );
}

#[tokio::test]
async fn errors_keep_reported_usage_and_retries_get_independent_records() {
    let state = support::state();
    let model = Model {
        ending: Ending::Failure,
        entered: Arc::new(tokio::sync::Notify::new()),
    };
    for _ in 0..2 {
        assert!(
            measured_turn(&state, &model, request(), ModelOptions::default())
                .await
                .is_err()
        );
    }
    let snapshot = state.llm_runtime.usage(&UsageQuery::default());
    assert_eq!(snapshot.totals.calls, 2);
    assert_eq!(snapshot.totals.output_tokens, 4);
    assert!(
        snapshot
            .records
            .iter()
            .all(|record| record.status == "failed")
    );
    assert_ne!(snapshot.records[0].id, snapshot.records[1].id);
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("upstream error")
    );
}

#[tokio::test]
async fn dropping_an_in_flight_turn_records_cancellation_and_partial_usage() {
    let state = support::state();
    let entered = Arc::new(tokio::sync::Notify::new());
    let model = Model {
        ending: Ending::Pending,
        entered: entered.clone(),
    };
    let worker_state = state.clone();
    let worker = tokio::spawn(async move {
        measured_turn(&worker_state, &model, request(), ModelOptions::default()).await
    });
    entered.notified().await;
    assert_eq!(
        state.llm_runtime.usage(&UsageQuery::default()).records[0].status,
        "running"
    );
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let snapshot = state.llm_runtime.usage(&UsageQuery::default());
    assert_eq!(snapshot.records[0].status, "cancelled");
    assert_eq!(snapshot.records[0].usage.output_tokens, Some(2));
}

struct LegacyModel;
impl LanguageModel for LegacyModel {
    fn decide(&self, _: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async {
            Ok(AgentDecision {
                reply_to: vec![],
                text: None,
                topic: None,
            })
        })
    }
}
#[tokio::test]
async fn models_without_usage_report_unknown_consumption() {
    let state = support::state();
    measured_turn(&state, &LegacyModel, request(), ModelOptions::default())
        .await
        .unwrap();
    let snapshot = state.llm_runtime.usage(&UsageQuery::default());
    assert_eq!(snapshot.totals.unknown_usage_calls, 1);
    assert_eq!(snapshot.totals.unpriced_calls, 1);
    assert_eq!(snapshot.records[0].usage.input_tokens, None);
}

#[tokio::test]
async fn completed_and_cancelled_calls_survive_reopen_without_prompt_or_error_data() {
    use meowlive_server::llm_runtime::RuntimeStore;
    let directory =
        std::env::temp_dir().join(format!("meowlive-metering-{}", uuid::Uuid::new_v4()));
    let mut state = support::state();
    state.llm_runtime = Arc::new(RuntimeStore::open(&directory).unwrap());
    let entered = Arc::new(tokio::sync::Notify::new());
    let model = Model {
        ending: Ending::Failure,
        entered: entered.clone(),
    };
    assert!(
        measured_turn(&state, &model, request(), ModelOptions::default())
            .await
            .is_err()
    );
    let entered = Arc::new(tokio::sync::Notify::new());
    let model = Model {
        ending: Ending::Pending,
        entered: entered.clone(),
    };
    let worker_state = state.clone();
    let worker = tokio::spawn(async move {
        measured_turn(&worker_state, &model, request(), ModelOptions::default()).await
    });
    entered.notified().await;
    worker.abort();
    let _ = worker.await;
    drop(state);
    let reopened = RuntimeStore::open(&directory).unwrap();
    let snapshot = reopened.usage(&UsageQuery::default());
    assert_eq!(snapshot.totals.calls, 2);
    assert_eq!(snapshot.totals.output_tokens, 4);
    assert!(
        snapshot
            .records
            .iter()
            .any(|record| record.status == "cancelled")
    );
    assert!(
        snapshot
            .records
            .iter()
            .any(|record| record.status == "failed")
    );
    for entry in std::fs::read_dir(&directory).unwrap() {
        let content = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!content.contains("private-prompt-marker"));
        assert!(!content.contains("fixture upstream error"));
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn a_full_history_segment_rotates_without_losing_earlier_totals() {
    use meowlive_server::llm_runtime::RuntimeStore;
    use serde_json::json;
    let directory =
        std::env::temp_dir().join(format!("meowlive-rotation-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let mut history = String::new();
    for index in 0..256 {
        let mut line=json!({"id":format!("historical-{index}"),"started_at_ms":index,"provider":"custom","api_format":"openai_chat",
            "base_url":"","model":"","operation":"agent","status":"completed","latency_ms":1,"first_token_ms":null,
            "usage":{"input_tokens":1,"output_tokens":1,"cache_read_tokens":0,"cache_write_tokens":0,"reasoning_tokens":0},
            "estimated_cost_microusd":null}).to_string();
        line.push_str(&" ".repeat(16 * 1024 - 1 - line.len()));
        line.push('\n');
        history.push_str(&line);
    }
    std::fs::write(directory.join("usage-00000000.jsonl"), history).unwrap();
    let mut state = support::state();
    state.llm_runtime = Arc::new(RuntimeStore::open(&directory).unwrap());
    measured_turn(&state, &LegacyModel, request(), ModelOptions::default())
        .await
        .unwrap();
    drop(state);
    let reopened = RuntimeStore::open(&directory).unwrap();
    let snapshot = reopened.usage(&UsageQuery::default());
    assert_eq!(snapshot.totals.calls, 257);
    assert_eq!(snapshot.totals.input_tokens, 256);
    assert_eq!(snapshot.records.len(), 200);
    assert!(snapshot.storage_available);
    assert!(
        std::fs::metadata(directory.join("usage-00000001.jsonl"))
            .unwrap()
            .len()
            > 0
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn runtime_storage_failure_is_visible_without_preventing_the_model_response() {
    use meowlive_server::llm_runtime::RuntimeStore;
    let directory =
        std::env::temp_dir().join(format!("meowlive-storage-failure-{}", uuid::Uuid::new_v4()));
    let mut state = support::state();
    state.llm_runtime = Arc::new(RuntimeStore::open(&directory).unwrap());
    std::fs::remove_file(directory.join("pending.json")).unwrap();
    std::fs::create_dir(directory.join("pending.json")).unwrap();
    measured_turn(&state, &LegacyModel, request(), ModelOptions::default())
        .await
        .unwrap();
    let snapshot = state.llm_runtime.usage(&UsageQuery::default());
    assert_eq!(snapshot.totals.calls, 1);
    assert_eq!(snapshot.records[0].status, "completed");
    assert!(!snapshot.storage_available);
    assert!(snapshot.truncated);
    std::fs::remove_dir_all(directory).unwrap();
}
