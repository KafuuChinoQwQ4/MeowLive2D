//! 领域对象与跨进程 DTO 的显式映射。
use meowlive_domain::speech::{SpeechReceiptStatus, SpeechStatus as DomainStatus, SpeechTask};
use meowlive_protocol::{
    control::{SpeechSnapshot, SpeechStatus},
    execution::ExecutionStatus,
};

pub fn speech(task: &SpeechTask) -> SpeechSnapshot {
    let status = match task.status {
        DomainStatus::Queued => SpeechStatus::Queued,
        DomainStatus::Synthesizing => SpeechStatus::Synthesizing,
        DomainStatus::Ready => SpeechStatus::Ready,
        DomainStatus::Playing => SpeechStatus::Playing,
        DomainStatus::Completed => SpeechStatus::Completed,
        DomainStatus::Cancelled => SpeechStatus::Cancelled,
        DomainStatus::Failed => SpeechStatus::Failed,
        DomainStatus::Unknown => SpeechStatus::Unknown,
    };
    SpeechSnapshot {
        id: task.id.clone(),
        generation: task.generation,
        text: task.text.as_str().to_owned(),
        voice_id: task.voice_id.as_str().to_owned(),
        status,
        error: task.error.clone(),
    }
}

pub fn receipt(status: ExecutionStatus) -> SpeechReceiptStatus {
    match status {
        ExecutionStatus::Started => SpeechReceiptStatus::Started,
        ExecutionStatus::Completed => SpeechReceiptStatus::Completed,
        ExecutionStatus::Cancelled => SpeechReceiptStatus::Cancelled,
        ExecutionStatus::Failed => SpeechReceiptStatus::Failed,
    }
}
