//! Agent 状态快照与服务端调用结果。
use super::AgentSettings;
use crate::ports::llm::DecisionRequest;
use meowlive_domain::event::LiveEvent;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentPhase {
    Paused,
    Waiting,
    Deciding,
    Speaking,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventStatus {
    Pending,
    Deciding,
    Skipped,
    Expired,
    Queued,
    Synthesizing,
    Ready,
    Playing,
    Completed,
    Cancelled,
    Failed,
    Unknown,
}
impl EventStatus {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Skipped
                | Self::Expired
                | Self::Completed
                | Self::Cancelled
                | Self::Failed
                | Self::Unknown
        )
    }
}

#[derive(Clone, Debug)]
pub struct EventRecord {
    pub event: LiveEvent,
    pub(crate) expires_at_ms: u64,
    pub status: EventStatus,
    pub speech_id: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub struct AgentView {
    pub paused: bool,
    pub phase: AgentPhase,
    pub settings: AgentSettings,
    pub events: Vec<EventRecord>,
    pub last_error: Option<String>,
    pub current_speech_id: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionWork {
    pub id: u64,
    pub request: DecisionRequest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentWaitReason {
    Paused,
    DecisionInFlight,
    SpeechInFlight,
    Cooldown { remaining_ms: u64 },
    CompletionBufferFull,
    WorkIdExhausted,
    NoEligibleEvents,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeginDecision {
    Work(DecisionWork),
    Waiting(AgentWaitReason),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedSpeech {
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionResolution {
    Speech(PreparedSpeech),
    Silent,
    Expired,
    PolicySkipped,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitOutcome {
    Accepted,
    Duplicate,
}

/// Actual playback completion; consumers drain this for persistent accounting.
#[derive(Clone, Debug)]
pub struct CompletedInteraction {
    pub speech_id: String,
    pub events: Vec<LiveEvent>,
    pub assistant: String,
}
