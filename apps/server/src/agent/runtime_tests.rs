use super::*;
use meowlive_application::ports::{
    llm::DecisionFuture,
    llm_runtime::{ModelTurnFuture, TokenUsage, ToolCall},
    speech::{SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
    web_search::{SearchFuture, SearchResult, WebSearch},
};
use meowlive_protocol::agent_observability::{
    AgentSchedulerBlockReason, AgentTraceStepKind, AgentTraceStepStatus,
};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

struct NoSpeech;
impl SpeechSynthesizer for NoSpeech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async { panic!("runtime must not synthesize") })
    }
}
fn state() -> AppState {
    AppState::new(crate::config::AppConfig::default(), Arc::new(NoSpeech))
}
fn request() -> DecisionRequest {
    DecisionRequest {
        persona: "测试主播".into(),
        topic: "聊天".into(),
        events: vec![],
        history: vec![],
        memory_context: vec![],
    }
}

struct Search;
impl WebSearch for Search {
    fn search(&self, query: String) -> SearchFuture<'_> {
        Box::pin(async move {
            assert_eq!(query, "新梗");
            Ok(vec![SearchResult {
                title: "来源".into(),
                url: "https://example.com/story".into(),
                snippet: "公开解释".into(),
            }])
        })
    }
}
struct ToolModel {
    calls: AtomicUsize,
    options: Mutex<Vec<ModelOptions>>,
    always_tool: bool,
}
impl LanguageModel for ToolModel {
    fn decide(&self, _: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async { panic!("must use native turn") })
    }
    fn turn(&self, _: DecisionRequest, options: ModelOptions) -> ModelTurnFuture<'_> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        self.options.lock().unwrap().push(options.clone());
        let always_tool = self.always_tool;
        Box::pin(async move {
            if let Some(observer) = options.observer {
                observer.on_event(ModelEvent::FirstToken);
                observer.on_event(ModelEvent::OutputProgress { characters: 12 });
            }
            Ok(if index == 0 || always_tool {
                ModelTurn {
                    decision: None,
                    tool_calls: vec![ToolCall {
                        id: format!("call-{index}"),
                        name: "web_search".into(),
                        arguments_json: r#"{"query":"新梗"}"#.into(),
                    }],
                    continuation: Some("opaque-native-signature".into()),
                    usage: TokenUsage::default(),
                    finish_reason: "tool_calls".into(),
                }
            } else {
                ModelTurn {
                    decision: Some(AgentDecision {
                        reply_to: vec![],
                        text: Some("根据刚查到的资料，这个梗是……".into()),
                        topic: None,
                    }),
                    tool_calls: vec![],
                    continuation: None,
                    usage: TokenUsage::default(),
                    finish_reason: "stop".into(),
                }
            })
        })
    }
}

#[tokio::test]
async fn search_results_return_with_native_continuation_and_activity_sources() {
    let mut state = state();
    let config = Arc::make_mut(&mut state.config);
    config.llm.reasoning_effort = "ultra".into();
    config.llm.provider = "anthropic".into();
    let mut settings = state.llm_runtime.settings();
    settings.web_search_enabled = true;
    let tools = ToolSet::with_search(&settings, Some(Arc::new(Search)));
    let model = ToolModel {
        calls: AtomicUsize::new(0),
        options: Mutex::new(vec![]),
        always_tool: false,
    };
    let activity = ActivityGuard::new(&state);
    let result = run(&state, &model, request(), 0, &settings, tools, &activity)
        .await
        .unwrap();
    assert!(result.text.unwrap().contains("刚查到"));
    let options = model.options.lock().unwrap();
    assert_eq!(options.len(), 2);
    for option in options.iter() {
        assert_eq!(option.reasoning_effort.as_str(), "ultra");
        assert_eq!(option.reasoning_provider, "anthropic");
    }
    assert_eq!(
        options[1].exchanges[0].continuation,
        "opaque-native-signature"
    );
    assert!(
        options[1].exchanges[0].results[0]
            .content
            .contains("公开解释")
    );
    assert_eq!(
        state.llm_runtime.activity().tools[0].sources,
        vec!["https://example.com/story"]
    );
}

#[tokio::test]
async fn repeated_tool_requests_are_bounded() {
    let state = state();
    let mut settings = state.llm_runtime.settings();
    settings.web_search_enabled = true;
    settings.max_tool_rounds = 1;
    let tools = ToolSet::with_search(&settings, Some(Arc::new(Search)));
    let model = ToolModel {
        calls: AtomicUsize::new(0),
        options: Mutex::new(vec![]),
        always_tool: true,
    };
    let activity = ActivityGuard::new(&state);
    assert!(
        run(&state, &model, request(), 0, &settings, tools, &activity)
            .await
            .unwrap_err()
            .message
            .contains("轮次")
    );
    assert_eq!(model.calls.load(Ordering::SeqCst), 2);
    assert!(model.options.lock().unwrap()[1].tool_choice_none);
}

#[tokio::test]
async fn only_enabled_read_only_tools_accept_strict_arguments() {
    let state = state();
    let settings = state.llm_runtime.settings();
    let tools = ToolSet::new(&settings, None);
    for (name, args) in [
        ("get_obs_status", r#"{"operation":"start_recording"}"#),
        ("shell", r#"{"command":"pwd"}"#),
        ("web_search", r#"{"query":"test"}"#),
    ] {
        let (result, _) = tools
            .execute(
                &state,
                &ToolCall {
                    id: "1".into(),
                    name: name.into(),
                    arguments_json: args.into(),
                },
            )
            .await;
        assert!(result.is_error, "{name}");
    }
    let (result, _) = tools
        .execute(
            &state,
            &ToolCall {
                id: "2".into(),
                name: "get_environment".into(),
                arguments_json: "{}".into(),
            },
        )
        .await;
    let value: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(value["desktop_connected"], false);
    assert_eq!(value["live"]["phase"], "disabled");
    assert!(value["time"]["utc"].as_str().unwrap().contains('T'));
}

#[test]
fn cancellation_marks_activity_without_overwriting_a_new_run() {
    let state = state();
    let first = ActivityGuard::new(&state);
    let first_id = first.id.clone();
    drop(first);
    assert_eq!(state.llm_runtime.activity().phase, "cancelled");
    let old = ActivityGuard::new(&state);
    let new = ActivityGuard::new(&state);
    drop(old);
    assert_eq!(
        state.llm_runtime.activity().run_id.as_deref(),
        Some(new.id.as_str())
    );
    assert_eq!(state.llm_runtime.activity().phase, "thinking");
    assert_ne!(first_id, new.id);
}

struct WaitingSearch(Arc<AtomicUsize>);
impl WebSearch for WaitingSearch {
    fn search(&self, _: String) -> SearchFuture<'_> {
        let dropped = self.0.clone();
        Box::pin(async move {
            struct Guard(Arc<AtomicUsize>);
            impl Drop for Guard {
                fn drop(&mut self) {
                    self.0.fetch_add(1, Ordering::SeqCst);
                }
            }
            let _guard = Guard(dropped);
            std::future::pending().await
        })
    }
}

#[tokio::test(start_paused = true)]
async fn tool_timeout_returns_a_failure_result_for_the_model_to_clarify() {
    let state = state();
    let mut settings = state.llm_runtime.settings();
    settings.web_search_enabled = true;
    settings.tool_timeout_seconds = 1;
    let dropped = Arc::new(AtomicUsize::new(0));
    let tools = ToolSet::with_search(&settings, Some(Arc::new(WaitingSearch(dropped.clone()))));
    let model = ToolModel {
        calls: AtomicUsize::new(0),
        options: Mutex::new(vec![]),
        always_tool: false,
    };
    let activity = ActivityGuard::new(&state);
    run(&state, &model, request(), 0, &settings, tools, &activity)
        .await
        .unwrap();
    let options = model.options.lock().unwrap();
    assert!(options[1].exchanges[0].results[0].is_error);
    assert!(options[1].exchanges[0].results[0].content.contains("超时"));
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert_eq!(state.llm_runtime.activity().tools[0].status, "failed");
}

#[tokio::test]
async fn aborting_a_tool_round_drops_io_and_never_starts_the_followup_turn() {
    let state = state();
    let view = state.clone();
    let trace_id = state.agent_observability.start_trace("proactive", vec![]);
    let task_trace_id = trace_id.clone();
    let mut settings = state.llm_runtime.settings();
    settings.web_search_enabled = true;
    let dropped = Arc::new(AtomicUsize::new(0));
    let dropped_search = dropped.clone();
    let model = Arc::new(ToolModel {
        calls: AtomicUsize::new(0),
        options: Mutex::new(vec![]),
        always_tool: false,
    });
    let task_model = model.clone();
    let task = tokio::spawn(async move {
        let activity = ActivityGuard::new(&state);
        run_observed(
            &state,
            task_model.as_ref(),
            request(),
            0,
            &settings,
            ToolSet::with_search(&settings, Some(Arc::new(WaitingSearch(dropped_search)))),
            &activity,
            Some(&task_trace_id),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while view.llm_runtime.activity().phase != "tool" {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(model.calls.load(Ordering::SeqCst), 1);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert_eq!(view.llm_runtime.activity().phase, "cancelled");
    let trace = view.agent_observability.get(&trace_id).unwrap();
    assert!(trace.steps.iter().any(|step| {
        step.kind == AgentTraceStepKind::ToolFinished
            && step.status == AgentTraceStepStatus::Cancelled
    }));
}

struct UnknownToolModel(AtomicUsize);
impl LanguageModel for UnknownToolModel {
    fn decide(&self, _: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async { panic!("must use native turn") })
    }
    fn turn(&self, _: DecisionRequest, _: ModelOptions) -> ModelTurnFuture<'_> {
        let call = self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            Ok(if call == 0 {
                ModelTurn {
                    decision: None,
                    tool_calls: vec![ToolCall {
                        id: "unknown-1".into(),
                        name: "PRIVATE_PROMPT_FRAGMENT".into(),
                        arguments_json: "{}".into(),
                    }],
                    continuation: Some("opaque".into()),
                    usage: TokenUsage::default(),
                    finish_reason: "tool_calls".into(),
                }
            } else {
                ModelTurn {
                    decision: Some(AgentDecision {
                        reply_to: vec![],
                        text: Some("完成".into()),
                        topic: None,
                    }),
                    tool_calls: vec![],
                    continuation: None,
                    usage: TokenUsage::default(),
                    finish_reason: "stop".into(),
                }
            })
        })
    }
}

#[tokio::test]
async fn unknown_model_tool_names_are_redacted_from_observation() {
    let state = state();
    let settings = state.llm_runtime.settings();
    let tools = ToolSet::new(&settings, None);
    let trace_id = state.agent_observability.start_trace("proactive", vec![]);
    let activity = ActivityGuard::new(&state);
    run_observed(
        &state,
        &UnknownToolModel(AtomicUsize::new(0)),
        request(),
        0,
        &settings,
        tools,
        &activity,
        Some(&trace_id),
    )
    .await
    .unwrap();
    let trace = state.agent_observability.get(&trace_id).unwrap();
    let encoded = serde_json::to_string(&trace).unwrap();
    assert!(!encoded.contains("PRIVATE_PROMPT_FRAGMENT"));
    assert!(trace.steps.iter().any(|step| {
        step.kind == AgentTraceStepKind::ToolStarted
            && step.tool_name.as_deref() == Some("unknown_tool")
    }));
}

#[tokio::test(start_paused = true)]
async fn total_deadline_ends_running_tools_and_marks_the_round_failed() {
    let state = state();
    let mut settings = state.llm_runtime.settings();
    settings.web_search_enabled = true;
    settings.tool_timeout_seconds = 8;
    let dropped = Arc::new(AtomicUsize::new(0));
    let tools = ToolSet::with_search(&settings, Some(Arc::new(WaitingSearch(dropped.clone()))));
    let model = ToolModel {
        calls: AtomicUsize::new(0),
        options: Mutex::new(vec![]),
        always_tool: false,
    };
    let mut activity = ActivityGuard::new(&state);
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        run(&state, &model, request(), 0, &settings, tools, &activity),
    )
    .await;
    assert!(result.is_err());
    activity.finish("failed", "总时限已到");
    assert_eq!(state.llm_runtime.activity().phase, "failed");
    assert_eq!(state.llm_runtime.activity().tools[0].status, "failed");
    assert_eq!(model.calls.load(Ordering::SeqCst), 1);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn admission_reports_the_first_real_scheduler_blocker() {
    let mut state = state();
    assert_eq!(
        crate::agent::admission::assess(&state).await.block_reason,
        AgentSchedulerBlockReason::Paused
    );
    {
        let mut inner = state.inner.lock().await;
        inner.agent.set_paused(false, state.now_ms());
    }
    assert_eq!(
        crate::agent::admission::assess(&state).await.block_reason,
        AgentSchedulerBlockReason::ModelUnavailable
    );
    state.model = Some(Arc::new(ToolModel {
        calls: AtomicUsize::new(0),
        options: Mutex::new(vec![]),
        always_tool: false,
    }));
    assert_eq!(
        crate::agent::admission::assess(&state).await.block_reason,
        AgentSchedulerBlockReason::BridgeDisconnected
    );
    {
        let mut inner = state.inner.lock().await;
        inner.queue.set_connected(true);
    }
    assert_eq!(
        crate::agent::admission::assess(&state).await.block_reason,
        AgentSchedulerBlockReason::NoEligibleEvents
    );
    state
        .resource_changing
        .store(true, std::sync::atomic::Ordering::Release);
    assert_eq!(
        crate::agent::admission::assess(&state).await.block_reason,
        AgentSchedulerBlockReason::ResourceChanging
    );
}
