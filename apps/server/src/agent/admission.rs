//! Agent worker 与观察接口共享的准入判断和用户可读阻塞原因。
use crate::state::{AppState, Inner};
use meowlive_application::agent::{AgentPhase as ApplicationPhase, AgentWaitReason};
use meowlive_protocol::{
    agent::AgentPhase,
    agent_observability::{AgentSchedulerBlockReason, AgentSchedulerSnapshot},
};
use std::sync::atomic::Ordering;

pub(crate) async fn assess(state: &AppState) -> AgentSchedulerSnapshot {
    let mut inner = state.inner.lock().await;
    assess_locked(state, &mut inner)
}

pub(crate) fn assess_locked(state: &AppState, inner: &mut Inner) -> AgentSchedulerSnapshot {
    let now = state.now_ms();
    state.sync_agent(inner);
    let waiting = inner.agent.waiting_reason(now);
    let view = inner.agent.view(now);
    let phase = match view.phase {
        ApplicationPhase::Paused => AgentPhase::Paused,
        ApplicationPhase::Waiting => AgentPhase::Waiting,
        ApplicationPhase::Deciding => AgentPhase::Deciding,
        ApplicationPhase::Speaking => AgentPhase::Speaking,
    };
    let pending_events = view
        .events
        .iter()
        .filter(|event| event.status == meowlive_application::agent::EventStatus::Pending)
        .count()
        .min(u32::MAX as usize) as u32;
    let deciding_events = view
        .events
        .iter()
        .filter(|event| event.status == meowlive_application::agent::EventStatus::Deciding)
        .count()
        .min(u32::MAX as usize) as u32;
    let active_speeches = u32::from(view.current_speech_id.is_some());

    let (block_reason, remaining_ms) = if matches!(waiting, Some(AgentWaitReason::Paused)) {
        (AgentSchedulerBlockReason::Paused, None)
    } else if state.model.is_none() {
        (AgentSchedulerBlockReason::ModelUnavailable, None)
    } else if !inner.queue.is_connected() {
        (AgentSchedulerBlockReason::BridgeDisconnected, None)
    } else if state.resource_changing.load(Ordering::Acquire) {
        (AgentSchedulerBlockReason::ResourceChanging, None)
    } else if state.gpu_busy.load(Ordering::Acquire) {
        (AgentSchedulerBlockReason::GpuBusy, None)
    } else if state
        .model_synthesizer
        .as_ref()
        .is_some_and(|synthesizer| synthesizer.is_busy())
    {
        (AgentSchedulerBlockReason::SynthesizerBusy, None)
    } else if state.receipts_pending.load(Ordering::Acquire) >= 4095 {
        (AgentSchedulerBlockReason::ReceiptBacklog, None)
    } else if inner.queue.tasks().any(|task| !task.status.is_terminal()) {
        (AgentSchedulerBlockReason::PlaybackBusy, None)
    } else {
        match waiting {
            None => (AgentSchedulerBlockReason::Ready, None),
            Some(AgentWaitReason::Paused) => unreachable!("handled above"),
            Some(AgentWaitReason::DecisionInFlight) => {
                (AgentSchedulerBlockReason::DecisionInFlight, None)
            }
            Some(AgentWaitReason::SpeechInFlight) => {
                (AgentSchedulerBlockReason::SpeechInFlight, None)
            }
            Some(AgentWaitReason::Cooldown { remaining_ms }) => {
                (AgentSchedulerBlockReason::Cooldown, Some(remaining_ms))
            }
            Some(AgentWaitReason::CompletionBufferFull) => {
                (AgentSchedulerBlockReason::CompletionBufferFull, None)
            }
            Some(AgentWaitReason::WorkIdExhausted) => {
                (AgentSchedulerBlockReason::WorkIdExhausted, None)
            }
            Some(AgentWaitReason::NoEligibleEvents) => {
                (AgentSchedulerBlockReason::NoEligibleEvents, None)
            }
        }
    };
    AgentSchedulerSnapshot {
        ready: block_reason == AgentSchedulerBlockReason::Ready,
        phase,
        block_reason,
        message: message(block_reason).into(),
        remaining_ms,
        pending_events,
        deciding_events,
        active_speeches,
        updated_at_ms: crate::viewers::utc_ms(),
    }
}

fn message(reason: AgentSchedulerBlockReason) -> &'static str {
    match reason {
        AgentSchedulerBlockReason::Ready => "已有事件可以调度",
        AgentSchedulerBlockReason::Paused => "Agent 已暂停",
        AgentSchedulerBlockReason::ModelUnavailable => "LLM 尚未配置",
        AgentSchedulerBlockReason::BridgeDisconnected => "桌面执行端未连接",
        AgentSchedulerBlockReason::ResourceChanging => "正在切换角色或音色",
        AgentSchedulerBlockReason::GpuBusy => "GPU 正在执行其他任务",
        AgentSchedulerBlockReason::SynthesizerBusy => "语音模型正在合成",
        AgentSchedulerBlockReason::ReceiptBacklog => "播放回执正在积压",
        AgentSchedulerBlockReason::PlaybackBusy => "播放队列正在处理其他内容",
        AgentSchedulerBlockReason::DecisionInFlight => "已有 Agent 决策正在进行",
        AgentSchedulerBlockReason::SpeechInFlight => "正在等待本轮语音完成",
        AgentSchedulerBlockReason::Cooldown => "Agent 正在冷却",
        AgentSchedulerBlockReason::CompletionBufferFull => "完成回执缓冲区已满",
        AgentSchedulerBlockReason::WorkIdExhausted => "Agent 工作编号已耗尽",
        AgentSchedulerBlockReason::NoEligibleEvents => "正在等待新的可回应事件",
    }
}
