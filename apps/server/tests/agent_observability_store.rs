use meowlive_protocol::agent_observability::{
    AgentTraceEvent, AgentTraceStatus, AgentTraceStepKind, AgentTraceStepStatus,
};
use meowlive_protocol::llm_runtime::LlmTokenUsage;
use meowlive_server::agent_observability::{AgentTraceStore, TraceStepInput, TurnInput};
use std::path::PathBuf;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("meowlive-traces-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn event(index: usize) -> AgentTraceEvent {
    AgentTraceEvent {
        id: format!("event-{index}"),
        kind: "chat".into(),
        viewer: "小猫".into(),
        summary: "晚上好".into(),
    }
}

fn turn(index: u32) -> TurnInput {
    TurnInput {
        tool_round: index,
        retry_attempt: 0,
        provider: "fixture".into(),
        api_format: "openai_chat".into(),
        model: "test-model".into(),
    }
}

#[test]
fn trace_state_is_owned_bounded_and_ordered() {
    let store = AgentTraceStore::memory();
    let trace_id = store.start_trace("live_events", (0..17).map(event).collect());
    let first_turn = store.start_turn(&trace_id, turn(0)).unwrap();
    for index in 1..17 {
        let _ = store.start_turn(&trace_id, turn(index));
    }
    assert!(store.start_turn("another-trace", turn(0)).is_none());
    store.finish_turn(
        &trace_id,
        &first_turn,
        AgentTraceStatus::Completed,
        Some(12),
        40,
        LlmTokenUsage {
            input_tokens: Some(10),
            output_tokens: Some(2),
            ..Default::default()
        },
    );
    store.finish_turn(
        "another-trace",
        &first_turn,
        AgentTraceStatus::Failed,
        None,
        1,
        LlmTokenUsage::default(),
    );
    for index in 0..100 {
        store.append_step(
            &trace_id,
            TraceStepInput {
                kind: AgentTraceStepKind::ToolFinished,
                status: AgentTraceStepStatus::Completed,
                message: format!("工具 {index}"),
                turn_id: Some(first_turn.clone()),
                tool_name: Some("web_search".into()),
                speech_id: None,
                elapsed_ms: Some(1),
                sources: vec!["https://example.com/source".into()],
            },
        );
    }
    for index in 0..300 {
        store.append_step(
            &trace_id,
            TraceStepInput {
                kind: AgentTraceStepKind::ContextReady,
                status: AgentTraceStepStatus::Completed,
                message: format!("上下文 {index}"),
                turn_id: None,
                tool_name: None,
                speech_id: None,
                elapsed_ms: None,
                sources: vec![],
            },
        );
    }
    let active = store.get(&trace_id).unwrap();
    assert!(active.steps[0].message.contains("弹幕公平调度"));
    assert_eq!(active.events.len(), 16);
    assert_eq!(active.turns.len(), 16);
    assert_eq!(active.steps.len(), 256);
    assert_eq!(
        active
            .steps
            .iter()
            .filter(|step| matches!(
                step.kind,
                AgentTraceStepKind::ToolStarted | AgentTraceStepKind::ToolFinished
            ))
            .count(),
        64
    );
    assert!(active.summary.truncated);
    assert!(
        active
            .steps
            .windows(2)
            .all(|steps| steps[0].sequence < steps[1].sequence)
    );
    assert_eq!(active.turns[0].status, AgentTraceStatus::Completed);
}

#[test]
fn speech_state_is_recorded_once_and_terminal_trace_rejects_late_steps() {
    let store = AgentTraceStore::memory();
    let trace_id = store.start_trace("proactive", vec![]);
    store.bind_speech(&trace_id, "speech-1");
    store.record_speech(
        &trace_id,
        "speech-1",
        AgentTraceStepKind::SpeechPlaying,
        "正在播放",
    );
    store.record_speech(
        &trace_id,
        "speech-1",
        AgentTraceStepKind::SpeechPlaying,
        "正在播放",
    );
    store.finish_trace(&trace_id, AgentTraceStatus::Completed, "播放完成");
    let before = store.get(&trace_id).unwrap();
    store.append_step(
        &trace_id,
        TraceStepInput {
            kind: AgentTraceStepKind::ToolStarted,
            status: AgentTraceStepStatus::Running,
            message: "late".into(),
            turn_id: None,
            tool_name: Some("web_search".into()),
            speech_id: None,
            elapsed_ms: None,
            sources: vec![],
        },
    );
    let after = store.get(&trace_id).unwrap();
    assert_eq!(after.summary.status, AgentTraceStatus::Completed);
    assert_eq!(before.steps, after.steps);
    assert_eq!(
        after
            .steps
            .iter()
            .filter(|step| step.kind == AgentTraceStepKind::SpeechPlaying)
            .count(),
        1
    );
}

#[test]
fn restart_interrupts_active_trace_and_keeps_completed_history() {
    let directory = Directory::new();
    let first_id = {
        let store = AgentTraceStore::open(&directory.0).unwrap();
        let id = store.start_trace("live_events", vec![event(1)]);
        store.append_step(
            &id,
            TraceStepInput {
                kind: AgentTraceStepKind::ContextReady,
                status: AgentTraceStepStatus::Completed,
                message: "上下文已准备".into(),
                turn_id: None,
                tool_name: None,
                speech_id: None,
                elapsed_ms: None,
                sources: vec![],
            },
        );
        id
    };
    let completed_id;
    {
        let store = AgentTraceStore::open(&directory.0).unwrap();
        assert_eq!(
            store.get(&first_id).unwrap().summary.status,
            AgentTraceStatus::Interrupted
        );
        completed_id = store.start_trace("proactive", vec![]);
        store.finish_trace(&completed_id, AgentTraceStatus::Completed, "无需播报");
    }
    let reopened = AgentTraceStore::open(&directory.0).unwrap();
    let list = reopened.list(10, None);
    assert_eq!(list.traces.len(), 2);
    assert!(list.storage_available);
    assert_eq!(
        reopened.get(&completed_id).unwrap().summary.status,
        AgentTraceStatus::Completed
    );
}

#[test]
fn restart_interrupts_a_trace_that_already_reached_the_step_limit() {
    let directory = Directory::new();
    let trace_id = {
        let store = AgentTraceStore::open(&directory.0).unwrap();
        let id = store.start_trace("proactive", vec![]);
        for index in 0..300 {
            store.append_step(
                &id,
                TraceStepInput {
                    kind: AgentTraceStepKind::ContextReady,
                    status: AgentTraceStepStatus::Completed,
                    message: format!("上下文 {index}"),
                    turn_id: None,
                    tool_name: None,
                    speech_id: None,
                    elapsed_ms: None,
                    sources: vec![],
                },
            );
        }
        assert_eq!(store.get(&id).unwrap().steps.len(), 256);
        id
    };
    let reopened = AgentTraceStore::open(&directory.0).unwrap();
    let recovered = reopened.get(&trace_id).unwrap();
    assert_eq!(recovered.summary.status, AgentTraceStatus::Interrupted);
    assert!(recovered.summary.truncated);
    assert!(recovered.steps.len() <= 256);
}

#[test]
fn malformed_active_or_history_file_stops_recovery() {
    for file in ["active.json", "trace-00000000.jsonl"] {
        let directory = Directory::new();
        std::fs::write(directory.0.join(file), b"{broken fixture").unwrap();
        assert!(AgentTraceStore::open(&directory.0).is_err(), "{file}");
        assert_eq!(
            std::fs::read(directory.0.join(file)).unwrap(),
            b"{broken fixture"
        );
    }
}

#[test]
fn storage_failure_does_not_block_traces_and_bounds_fallback_history() {
    let directory = Directory::new();
    let store = AgentTraceStore::open(&directory.0).unwrap();
    std::fs::create_dir(directory.0.join("active.tmp")).unwrap();
    let first = store.start_trace("proactive", vec![]);
    store.finish_trace(&first, AgentTraceStatus::Completed, "完成");
    let mut latest = String::new();
    for _ in 0..104 {
        latest = store.start_trace("proactive", vec![]);
        store.finish_trace(&latest, AgentTraceStatus::Completed, "完成");
    }
    let list = store.list(100, None);
    assert!(!list.storage_available);
    assert_eq!(list.traces.len(), 100);
    assert!(store.get(&first).is_none());
    assert!(store.get(&latest).is_some());
}

#[test]
fn unavailable_storage_at_startup_degrades_to_bounded_memory() {
    let directory = Directory::new();
    let file = directory.0.join("not-a-directory");
    std::fs::write(&file, b"fixture").unwrap();
    let store = AgentTraceStore::open(file.join("traces")).unwrap();
    let id = store.start_trace("proactive", vec![]);
    store.finish_trace(&id, AgentTraceStatus::Completed, "完成");
    let list = store.list(10, None);
    assert!(!list.storage_available);
    assert_eq!(list.traces[0].id, id);
}

#[test]
fn legal_large_sources_are_truncated_without_disabling_persistence() {
    let directory = Directory::new();
    let store = AgentTraceStore::open(&directory.0).unwrap();
    let id = store.start_trace("proactive", vec![]);
    for _ in 0..4 {
        store.append_step(
            &id,
            TraceStepInput {
                kind: AgentTraceStepKind::ToolFinished,
                status: AgentTraceStepStatus::Completed,
                message: "搜索完成".into(),
                turn_id: None,
                tool_name: Some("web_search".into()),
                speech_id: None,
                elapsed_ms: Some(1),
                sources: vec![format!("https://example.com/{}", "x".repeat(4000)); 32],
            },
        );
    }
    assert!(store.list(10, None).storage_available);
    assert!(store.get(&id).unwrap().summary.truncated);
    assert!(
        std::fs::metadata(directory.0.join("active.json"))
            .unwrap()
            .len()
            <= 256 * 1024
    );
    store.finish_trace(&id, AgentTraceStatus::Completed, "无需播报");
    let recovered = AgentTraceStore::open(&directory.0)
        .unwrap()
        .get(&id)
        .unwrap();
    assert_eq!(recovered.summary.status, AgentTraceStatus::Completed);
    assert!(recovered.summary.truncated);
}

#[test]
fn legal_multiline_events_have_single_line_observation_summaries() {
    let store = AgentTraceStore::memory();
    let id = store.start_trace(
        "live_events",
        vec![AgentTraceEvent {
            summary: "第一行\n第二行\t说明".into(),
            ..event(1)
        }],
    );
    assert_eq!(
        store.get(&id).unwrap().events[0].summary,
        "第一行 第二行 说明"
    );
}

#[test]
fn terminal_active_file_is_rejected_without_rewriting_it() {
    let directory = Directory::new();
    let store = AgentTraceStore::memory();
    let id = store.start_trace("proactive", vec![]);
    store.finish_trace(&id, AgentTraceStatus::Completed, "完成");
    let bytes = serde_json::to_vec(&store.get(&id).unwrap()).unwrap();
    std::fs::write(directory.0.join("active.json"), &bytes).unwrap();
    assert!(AgentTraceStore::open(&directory.0).is_err());
    assert_eq!(
        std::fs::read(directory.0.join("active.json")).unwrap(),
        bytes
    );
}

#[test]
fn each_detail_change_advances_the_polling_cursor_even_within_one_millisecond() {
    let store = AgentTraceStore::memory();
    let id = store.start_trace("proactive", vec![]);
    let mut previous = store.get(&id).unwrap().summary.updated_at_ms;
    for _ in 0..20 {
        store.append_step(
            &id,
            TraceStepInput {
                kind: AgentTraceStepKind::ContextReady,
                status: AgentTraceStepStatus::Completed,
                message: "已准备".into(),
                turn_id: None,
                tool_name: None,
                speech_id: None,
                elapsed_ms: None,
                sources: vec![],
            },
        );
        let next = store.get(&id).unwrap().summary.updated_at_ms;
        assert!(next > previous);
        previous = next;
    }
    store.finish_trace(&id, AgentTraceStatus::Completed, "无需播报");
    assert!(store.get(&id).unwrap().summary.updated_at_ms > previous);
}

#[test]
fn history_rotates_and_retires_whole_segments_at_the_record_limit() {
    let directory = Directory::new();
    let store = AgentTraceStore::open(&directory.0).unwrap();
    let mut first = String::new();
    let mut last = String::new();
    for index in 0..1_005 {
        let id = store.start_trace("proactive", vec![]);
        store.finish_trace(&id, AgentTraceStatus::Completed, "完成");
        if index == 0 {
            first = id.clone();
        }
        last = id;
    }
    let mut count = 0;
    for entry in std::fs::read_dir(&directory.0).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(text.len() <= 4 * 1024 * 1024);
            count += text.lines().count();
        }
    }
    assert_eq!(count, 905);
    let recovered = AgentTraceStore::open(&directory.0).unwrap();
    assert!(recovered.get(&first).is_none());
    assert!(recovered.get(&last).is_some());
}

#[test]
fn history_enforces_the_total_byte_limit_before_the_record_limit() {
    let directory = Directory::new();
    let store = AgentTraceStore::open(&directory.0).unwrap();
    let source = format!("https://example.com/{}", "x".repeat(4000));
    for _ in 0..150 {
        let id = store.start_trace("proactive", vec![]);
        for _ in 0..2 {
            store.append_step(
                &id,
                TraceStepInput {
                    kind: AgentTraceStepKind::ToolFinished,
                    status: AgentTraceStepStatus::Completed,
                    message: "搜索完成".into(),
                    turn_id: None,
                    tool_name: Some("web_search".into()),
                    speech_id: None,
                    elapsed_ms: Some(1),
                    sources: vec![source.clone(); 31],
                },
            );
        }
        store.finish_trace(&id, AgentTraceStatus::Completed, "完成");
    }
    assert!(store.list(10, None).storage_available);
    let mut total = 0;
    let mut segments = 0;
    for entry in std::fs::read_dir(&directory.0).unwrap() {
        let entry = entry.unwrap();
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            let bytes = entry.metadata().unwrap().len();
            assert!(bytes <= 4 * 1024 * 1024);
            total += bytes;
            segments += 1;
        }
    }
    assert!(segments > 1);
    assert!(total <= 32 * 1024 * 1024);
    assert!(!directory.0.join("trace-00000000.jsonl").exists());
}
