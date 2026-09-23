//! Agent 调度、模型 turn、工具和语音状态的有界观察记录。
mod persistence;

use meowlive_domain::speech::{SpeechStatus, SpeechTask};
use meowlive_protocol::{
    agent::AgentPhase,
    agent_observability::{
        AgentSchedulerBlockReason, AgentSchedulerSnapshot, AgentTrace, AgentTraceEvent,
        AgentTraceList, AgentTraceStatus, AgentTraceStep, AgentTraceStepKind, AgentTraceStepStatus,
        AgentTraceSummary, AgentTurn,
    },
    llm_runtime::LlmTokenUsage,
};
use persistence::{DiskOpenError, DiskTraceStore};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const MAX_EVENTS: usize = 16;
const MAX_TURNS: usize = 16;
const MAX_TOOL_STEPS: usize = 64;
const MAX_STEPS: usize = 256;
const MAX_PERSISTED_TRACES: usize = 1_000;
const MAX_FALLBACK_TRACES: usize = 100;

#[derive(Clone, Debug)]
pub struct TraceStepInput {
    pub kind: AgentTraceStepKind,
    pub status: AgentTraceStepStatus,
    pub message: String,
    pub turn_id: Option<String>,
    pub tool_name: Option<String>,
    pub speech_id: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TurnInput {
    pub tool_round: u32,
    pub retry_attempt: u32,
    pub provider: String,
    pub api_format: String,
    pub model: String,
}

pub struct AgentTraceStore {
    inner: Mutex<TraceState>,
}

struct TraceState {
    scheduler: AgentSchedulerSnapshot,
    active: Option<AgentTrace>,
    history: VecDeque<AgentTrace>,
    disk: Option<DiskTraceStore>,
    healthy: bool,
}

impl AgentTraceStore {
    pub fn memory() -> Self {
        Self {
            inner: Mutex::new(TraceState {
                scheduler: scheduler_default(),
                active: None,
                history: VecDeque::new(),
                disk: None,
                healthy: true,
            }),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        match DiskTraceStore::open(path.as_ref()) {
            Ok((disk, history)) => Ok(Self {
                inner: Mutex::new(TraceState {
                    scheduler: scheduler_default(),
                    active: None,
                    history: history.into(),
                    disk: Some(disk),
                    healthy: true,
                }),
            }),
            Err(DiskOpenError::Corrupt(message)) => Err(message),
            Err(DiskOpenError::Unavailable(_message)) => Ok(Self {
                inner: Mutex::new(TraceState {
                    scheduler: scheduler_default(),
                    active: None,
                    history: VecDeque::new(),
                    disk: None,
                    healthy: false,
                }),
            }),
        }
    }

    pub fn scheduler(&self) -> AgentSchedulerSnapshot {
        self.inner.lock().unwrap().scheduler.clone()
    }

    pub fn set_scheduler(&self, scheduler: AgentSchedulerSnapshot) {
        self.inner.lock().unwrap().scheduler = scheduler;
    }

    pub fn start_trace(&self, trigger: impl Into<String>, events: Vec<AgentTraceEvent>) -> String {
        let mut inner = self.inner.lock().unwrap();
        if let Some(mut active) = inner.active.take() {
            finish_value(
                &mut active,
                AgentTraceStatus::Cancelled,
                "新的任务已接替上一条观察记录",
            );
            persist_finished(&mut inner, active);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let trigger = bounded(trigger.into(), 64);
        let scheduled_message = scheduling_message(&trigger, &events);
        let truncated = events.len() > MAX_EVENTS;
        let mut trace = AgentTrace {
            summary: AgentTraceSummary {
                id: id.clone(),
                status: AgentTraceStatus::Running,
                trigger,
                started_at_ms: now,
                updated_at_ms: now,
                finished_at_ms: None,
                event_count: events.len().min(u32::MAX as usize) as u32,
                turn_count: 0,
                tool_count: 0,
                speech_id: None,
                result: "已进入 Agent 调度".into(),
                truncated,
            },
            events: events
                .into_iter()
                .take(MAX_EVENTS)
                .map(sanitize_event)
                .collect(),
            turns: Vec::new(),
            steps: Vec::new(),
        };
        push_step(
            &mut trace,
            TraceStepInput {
                kind: AgentTraceStepKind::Scheduled,
                status: AgentTraceStepStatus::Completed,
                message: scheduled_message,
                turn_id: None,
                tool_name: None,
                speech_id: None,
                elapsed_ms: None,
                sources: vec![],
            },
        );
        inner.active = Some(trace);
        persist_active(&mut inner);
        id
    }

    pub fn append_step(&self, trace_id: &str, input: TraceStepInput) {
        let mut inner = self.inner.lock().unwrap();
        let Some(trace) = active_for(&mut inner, trace_id) else {
            return;
        };
        push_step(trace, input);
        persist_active(&mut inner);
    }

    pub fn start_turn(&self, trace_id: &str, input: TurnInput) -> Option<String> {
        let mut inner = self.inner.lock().unwrap();
        let trace = active_for(&mut inner, trace_id)?;
        if trace.turns.len() >= MAX_TURNS {
            trace.summary.truncated = true;
            persist_active(&mut inner);
            return None;
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        trace.turns.push(AgentTurn {
            id: id.clone(),
            index: trace.turns.len() as u32 + 1,
            tool_round: input.tool_round,
            retry_attempt: input.retry_attempt,
            provider: bounded(input.provider, 64),
            api_format: bounded(input.api_format, 64),
            model: bounded(input.model, 128),
            status: AgentTraceStatus::Running,
            started_at_ms: now,
            first_token_ms: None,
            finished_at_ms: None,
            latency_ms: 0,
            usage: LlmTokenUsage::default(),
        });
        trace.summary.turn_count = trace.turns.len() as u32;
        push_step(
            trace,
            TraceStepInput {
                kind: AgentTraceStepKind::TurnStarted,
                status: AgentTraceStepStatus::Running,
                message: format!("开始第 {} 次模型调用", trace.turns.len()),
                turn_id: Some(id.clone()),
                tool_name: None,
                speech_id: None,
                elapsed_ms: None,
                sources: vec![],
            },
        );
        persist_active(&mut inner);
        Some(id)
    }

    pub fn first_token(&self, trace_id: &str, turn_id: &str, elapsed_ms: u64) {
        let mut inner = self.inner.lock().unwrap();
        let Some(trace) = active_for(&mut inner, trace_id) else {
            return;
        };
        let Some(turn) = trace.turns.iter_mut().find(|turn| turn.id == turn_id) else {
            return;
        };
        if turn.first_token_ms.is_some() || turn.status != AgentTraceStatus::Running {
            return;
        }
        turn.first_token_ms = Some(elapsed_ms);
        push_step(
            trace,
            TraceStepInput {
                kind: AgentTraceStepKind::FirstToken,
                status: AgentTraceStepStatus::Completed,
                message: "已收到模型首段输出".into(),
                turn_id: Some(turn_id.into()),
                tool_name: None,
                speech_id: None,
                elapsed_ms: Some(elapsed_ms),
                sources: vec![],
            },
        );
        persist_active(&mut inner);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_turn(
        &self,
        trace_id: &str,
        turn_id: &str,
        status: AgentTraceStatus,
        first_token_ms: Option<u64>,
        latency_ms: u64,
        usage: LlmTokenUsage,
    ) {
        if status == AgentTraceStatus::Running {
            return;
        }
        let mut inner = self.inner.lock().unwrap();
        let Some(trace) = active_for(&mut inner, trace_id) else {
            return;
        };
        let Some(turn) = trace.turns.iter_mut().find(|turn| turn.id == turn_id) else {
            return;
        };
        if turn.status != AgentTraceStatus::Running {
            return;
        }
        turn.status = status;
        turn.first_token_ms = turn.first_token_ms.or(first_token_ms);
        turn.finished_at_ms = Some(now_ms());
        turn.latency_ms = latency_ms;
        turn.usage = usage;
        push_step(
            trace,
            TraceStepInput {
                kind: AgentTraceStepKind::TurnFinished,
                status: step_status(status),
                message: match status {
                    AgentTraceStatus::Completed => "模型调用完成",
                    AgentTraceStatus::Failed => "模型调用失败",
                    AgentTraceStatus::Cancelled => "模型调用已取消",
                    AgentTraceStatus::Interrupted => "模型调用被进程中断",
                    AgentTraceStatus::Running => unreachable!(),
                }
                .into(),
                turn_id: Some(turn_id.into()),
                tool_name: None,
                speech_id: None,
                elapsed_ms: Some(latency_ms),
                sources: vec![],
            },
        );
        persist_active(&mut inner);
    }

    pub fn bind_speech(&self, trace_id: &str, speech_id: &str) {
        let mut inner = self.inner.lock().unwrap();
        let Some(trace) = active_for(&mut inner, trace_id) else {
            return;
        };
        trace.summary.speech_id = Some(bounded(speech_id, 128));
        push_step(
            trace,
            TraceStepInput {
                kind: AgentTraceStepKind::SpeechQueued,
                status: AgentTraceStepStatus::Completed,
                message: "回应已加入语音队列".into(),
                turn_id: None,
                tool_name: None,
                speech_id: Some(speech_id.into()),
                elapsed_ms: None,
                sources: vec![],
            },
        );
        persist_active(&mut inner);
    }

    pub fn record_speech(
        &self,
        trace_id: &str,
        speech_id: &str,
        kind: AgentTraceStepKind,
        message: &str,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let Some(trace) = active_for(&mut inner, trace_id) else {
            return;
        };
        if trace.summary.speech_id.as_deref() != Some(speech_id)
            || trace.steps.last().is_some_and(|step| {
                step.kind == kind && step.speech_id.as_deref() == Some(speech_id)
            })
        {
            return;
        }
        push_step(
            trace,
            TraceStepInput {
                kind,
                status: AgentTraceStepStatus::Running,
                message: message.into(),
                turn_id: None,
                tool_name: None,
                speech_id: Some(speech_id.into()),
                elapsed_ms: None,
                sources: vec![],
            },
        );
        persist_active(&mut inner);
    }

    pub fn sync_speech(&self, task: &SpeechTask) {
        let trace_id = {
            let inner = self.inner.lock().unwrap();
            inner.active.as_ref().and_then(|trace| {
                (trace.summary.speech_id.as_deref() == Some(task.id.as_str()))
                    .then(|| trace.summary.id.clone())
            })
        };
        let Some(trace_id) = trace_id else {
            return;
        };
        match task.status {
            SpeechStatus::Queued => {}
            SpeechStatus::Synthesizing => self.record_speech(
                &trace_id,
                &task.id,
                AgentTraceStepKind::SpeechSynthesizing,
                "正在合成语音",
            ),
            SpeechStatus::Ready => self.record_speech(
                &trace_id,
                &task.id,
                AgentTraceStepKind::SpeechReady,
                "语音已就绪，等待播放",
            ),
            SpeechStatus::Playing => self.record_speech(
                &trace_id,
                &task.id,
                AgentTraceStepKind::SpeechPlaying,
                "桌面执行端正在播放",
            ),
            SpeechStatus::Completed => {
                self.finish_trace(&trace_id, AgentTraceStatus::Completed, "播放完成")
            }
            SpeechStatus::Cancelled => {
                self.finish_trace(&trace_id, AgentTraceStatus::Cancelled, "播放已取消")
            }
            SpeechStatus::Failed => {
                self.finish_trace(&trace_id, AgentTraceStatus::Failed, "播放失败")
            }
            SpeechStatus::Unknown => self.finish_trace(
                &trace_id,
                AgentTraceStatus::Failed,
                "连接断开，播放结果未知",
            ),
        }
    }

    pub fn finish_trace(&self, trace_id: &str, status: AgentTraceStatus, result: &str) {
        if status == AgentTraceStatus::Running {
            return;
        }
        let mut inner = self.inner.lock().unwrap();
        if inner.active.as_ref().map(|trace| trace.summary.id.as_str()) != Some(trace_id) {
            return;
        }
        let mut trace = inner.active.take().expect("matched active trace");
        finish_value(&mut trace, status, result);
        persist_finished(&mut inner, trace);
    }

    pub fn finish_speech(&self, speech_id: &str, status: AgentTraceStatus, result: &str) {
        let trace_id = {
            let inner = self.inner.lock().unwrap();
            inner.active.as_ref().and_then(|trace| {
                (trace.summary.speech_id.as_deref() == Some(speech_id))
                    .then(|| trace.summary.id.clone())
            })
        };
        if let Some(trace_id) = trace_id {
            self.finish_trace(&trace_id, status, result);
        }
    }

    pub fn list(&self, limit: usize, before_ms: Option<u64>) -> AgentTraceList {
        let inner = self.inner.lock().unwrap();
        let maximum = limit.clamp(1, 100);
        let values = inner
            .active
            .iter()
            .chain(inner.history.iter().rev())
            .filter(|trace| before_ms.is_none_or(|before| trace.summary.updated_at_ms < before))
            .take(maximum + 1)
            .map(|trace| trace.summary.clone())
            .collect::<Vec<_>>();
        AgentTraceList {
            truncated: values.len() > maximum,
            traces: values.into_iter().take(maximum).collect(),
            storage_available: inner.healthy,
        }
    }

    pub fn get(&self, id: &str) -> Option<AgentTrace> {
        let inner = self.inner.lock().unwrap();
        inner
            .active
            .iter()
            .chain(inner.history.iter().rev())
            .find(|trace| trace.summary.id == id)
            .cloned()
    }
}

fn active_for<'a>(state: &'a mut TraceState, id: &str) -> Option<&'a mut AgentTrace> {
    state
        .active
        .as_mut()
        .filter(|trace| trace.summary.id == id && trace.summary.status == AgentTraceStatus::Running)
}

fn push_step(trace: &mut AgentTrace, input: TraceStepInput) {
    let tool_step = matches!(
        input.kind,
        AgentTraceStepKind::ToolStarted | AgentTraceStepKind::ToolFinished
    );
    let existing_tools = trace
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.kind,
                AgentTraceStepKind::ToolStarted | AgentTraceStepKind::ToolFinished
            )
        })
        .count();
    if trace.steps.len() >= MAX_STEPS || (tool_step && existing_tools >= MAX_TOOL_STEPS) {
        trace.summary.truncated = true;
        return;
    }
    let sequence = trace.steps.last().map_or(1, |step| step.sequence + 1);
    if input.kind == AgentTraceStepKind::ToolStarted {
        trace.summary.tool_count = trace.summary.tool_count.saturating_add(1);
    }
    let now = now_ms();
    trace.steps.push(AgentTraceStep {
        sequence,
        occurred_at_ms: now,
        kind: input.kind,
        status: input.status,
        message: bounded(input.message, 1_000),
        turn_id: input.turn_id.map(|value| bounded(value, 128)),
        tool_name: input.tool_name.map(|value| bounded(value, 128)),
        speech_id: input.speech_id.map(|value| bounded(value, 128)),
        elapsed_ms: input.elapsed_ms,
        sources: input
            .sources
            .into_iter()
            .filter(|value| safe_source(value))
            .take(32)
            .map(|value| bounded(value, 4_096))
            .collect(),
    });
}

fn finish_value(trace: &mut AgentTrace, status: AgentTraceStatus, result: &str) {
    let mut pending_tools = Vec::<AgentTraceStep>::new();
    for step in &trace.steps {
        if step.kind == AgentTraceStepKind::ToolStarted {
            pending_tools.push(step.clone());
        } else if step.kind == AgentTraceStepKind::ToolFinished {
            if let Some(index) = pending_tools.iter().position(|start| {
                start.turn_id == step.turn_id && start.tool_name == step.tool_name
            }) {
                pending_tools.remove(index);
            }
        }
    }
    for tool in pending_tools {
        push_step(
            trace,
            TraceStepInput {
                kind: AgentTraceStepKind::ToolFinished,
                status: if status == AgentTraceStatus::Cancelled {
                    AgentTraceStepStatus::Cancelled
                } else {
                    AgentTraceStepStatus::Failed
                },
                message: "任务已经结束，工具调用被中断".into(),
                turn_id: tool.turn_id,
                tool_name: tool.tool_name,
                speech_id: None,
                elapsed_ms: Some(now_ms().saturating_sub(tool.occurred_at_ms)),
                sources: vec![],
            },
        );
    }
    push_step(
        trace,
        TraceStepInput {
            kind: AgentTraceStepKind::TraceFinished,
            status: step_status(status),
            message: result.into(),
            turn_id: None,
            tool_name: None,
            speech_id: trace.summary.speech_id.clone(),
            elapsed_ms: Some(now_ms().saturating_sub(trace.summary.started_at_ms)),
            sources: vec![],
        },
    );
    let now = now_ms();
    trace.summary.status = status;
    trace.summary.result = bounded(result, 1_000);
    trace.summary.updated_at_ms = now.max(trace.summary.updated_at_ms.saturating_add(1));
    trace.summary.finished_at_ms = Some(now);
    for turn in &mut trace.turns {
        if turn.status == AgentTraceStatus::Running {
            turn.status = match status {
                AgentTraceStatus::Interrupted => AgentTraceStatus::Interrupted,
                _ => AgentTraceStatus::Cancelled,
            };
            turn.finished_at_ms = Some(now);
            turn.latency_ms = now.saturating_sub(turn.started_at_ms);
        }
    }
}

fn persist_active(state: &mut TraceState) {
    if let Some(active) = &mut state.active {
        // The timestamp doubles as the detail polling cursor, including changes
        // within one clock tick and on clocks that move backwards.
        active.summary.updated_at_ms = now_ms().max(active.summary.updated_at_ms.saturating_add(1));
        let _ = persistence::fit_record(active);
    }
    if !state.healthy {
        return;
    }
    if let (Some(disk), Some(active)) = (&state.disk, &state.active) {
        if disk.save_active(active).is_err() {
            state.healthy = false;
            trim_history(state, MAX_FALLBACK_TRACES);
        }
    }
}

fn persist_finished(state: &mut TraceState, mut trace: AgentTrace) {
    let _ = persistence::fit_record(&mut trace);
    if state.healthy {
        if let Some(disk) = &mut state.disk {
            if disk
                .append(&trace)
                .and_then(|_| disk.clear_active())
                .is_err()
            {
                state.healthy = false;
            }
        }
    }
    state.history.push_back(trace);
    let maximum = if state.disk.is_some() && state.healthy {
        MAX_PERSISTED_TRACES
    } else {
        MAX_FALLBACK_TRACES
    };
    trim_history(state, maximum);
}

fn trim_history(state: &mut TraceState, maximum: usize) {
    while state.history.len() > maximum {
        state.history.pop_front();
    }
}

fn sanitize_event(mut event: AgentTraceEvent) -> AgentTraceEvent {
    event.id = bounded(event.id, 128);
    event.kind = bounded(event.kind, 64);
    event.viewer = bounded(event.viewer, 240);
    event.summary = bounded(
        event
            .summary
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        240,
    );
    event
}

fn scheduling_message(trigger: &str, events: &[AgentTraceEvent]) -> String {
    if trigger == "proactive" {
        "空闲主动发言已进入调度".into()
    } else if events.iter().any(|event| event.kind == "super_chat") {
        "按 SC 优先策略选择本轮任务".into()
    } else if events.iter().any(|event| event.kind == "gift") {
        "按礼物优先策略选择本轮任务，符合条件的同观众礼物已合并".into()
    } else if events.iter().any(|event| event.kind == "room_enter") {
        "欢迎条件和冷却校验通过，选择本轮进房事件".into()
    } else {
        "按弹幕公平调度策略选择本轮任务".into()
    }
}

fn bounded(value: impl Into<String>, maximum: usize) -> String {
    value.into().chars().take(maximum).collect()
}

fn safe_source(value: &str) -> bool {
    if value.len() > 4_096
        || value.contains('\\')
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return false;
    }
    value.parse::<axum::http::Uri>().is_ok_and(|uri| {
        matches!(uri.scheme_str(), Some("http" | "https"))
            && uri.host().is_some_and(|host| !host.is_empty())
            && uri
                .authority()
                .is_some_and(|authority| !authority.as_str().contains('@'))
    })
}

fn step_status(status: AgentTraceStatus) -> AgentTraceStepStatus {
    match status {
        AgentTraceStatus::Running => AgentTraceStepStatus::Running,
        AgentTraceStatus::Completed => AgentTraceStepStatus::Completed,
        AgentTraceStatus::Failed | AgentTraceStatus::Interrupted => AgentTraceStepStatus::Failed,
        AgentTraceStatus::Cancelled => AgentTraceStepStatus::Cancelled,
    }
}

fn scheduler_default() -> AgentSchedulerSnapshot {
    AgentSchedulerSnapshot {
        ready: false,
        phase: AgentPhase::Paused,
        block_reason: AgentSchedulerBlockReason::Paused,
        message: "Agent 已暂停".into(),
        remaining_ms: None,
        pending_events: 0,
        deciding_events: 0,
        active_speeches: 0,
        updated_at_ms: now_ms(),
    }
}

pub(crate) fn now_ms() -> u64 {
    crate::viewers::utc_ms()
}
