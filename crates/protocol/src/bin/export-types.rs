use meowlive_protocol::{
    agent::*, agent_observability::*, audio::*, auth::*, control::*, execution::*, launcher::*,
    live::*, llm::*, llm_runtime::*, model_library::*, obs::*, resources::*, training::*,
    training_runtime::*,
};
use meowlive_protocol::{companionship, memory, relationships, viewer_merge, viewers};
use std::{env, fs, path::PathBuf, process::ExitCode};
use ts_rs::TS;

fn main() -> ExitCode {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/contracts/src/index.ts");
    let types = [
        ChatReadMode::decl(),
        InteractionSettings::decl(),
        viewer_merge::MergeViewerSummary::decl(),
        viewer_merge::ViewerMergePreview::decl(),
        viewer_merge::ViewerMergePreviewRequest::decl(),
        viewer_merge::ViewerMergeRequest::decl(),
        viewer_merge::ViewerMergeOutcome::decl(),
        relationships::RelationshipEntity::decl(),
        relationships::RelationshipEvidence::decl(),
        relationships::ViewerRelationship::decl(),
        relationships::RelationshipPage::decl(),
        relationships::RelationshipCreateRequest::decl(),
        relationships::RelationshipChangeRequest::decl(),
        relationships::KnowledgeMaintenanceRequest::decl(),
        relationships::GraphStatus::decl(),
        companionship::AffinityLedgerEntry::decl(),
        companionship::GiftLedgerEntry::decl(),
        companionship::CompanionshipDetail::decl(),
        companionship::AffinityAdjustmentRequest::decl(),
        companionship::AffinityReversalRequest::decl(),
        companionship::GiftConfirmationRequest::decl(),
        companionship::AdminMutationResult::decl(),
        companionship::CompanionshipHealth::decl(),
        companionship::ReceiptProblem::decl(),
        memory::MemoryEvidence::decl(),
        memory::ViewerMemory::decl(),
        memory::MemoryPage::decl(),
        memory::MemoryMutationRequest::decl(),
        memory::MemoryJobsStatus::decl(),
        AdminSessionRequest::decl(),
        AdminSessionStatus::decl(),
        AdminSessionToken::decl(),
        viewers::ViewerAlias::decl(),
        viewers::ViewerIdentity::decl(),
        viewers::ViewerSummary::decl(),
        viewers::PersistedViewerEvent::decl(),
        viewers::ViewerPage::decl(),
        viewers::ViewerEventPage::decl(),
        LlmPrice::decl(),
        AgentRuntimeSettings::decl(),
        AgentRuntimeSettingsSnapshot::decl(),
        AgentRuntimeSettingsRequest::decl(),
        LlmTokenUsage::decl(),
        LlmUsageRecord::decl(),
        LlmUsageTotals::decl(),
        LlmUsageGroup::decl(),
        LlmUsageSnapshot::decl(),
        AgentToolActivity::decl(),
        AgentActivitySnapshot::decl(),
        AgentSchedulerBlockReason::decl(),
        AgentSchedulerSnapshot::decl(),
        AgentTraceStatus::decl(),
        AgentTraceStepKind::decl(),
        AgentTraceStepStatus::decl(),
        AgentTraceStep::decl(),
        AgentTraceEvent::decl(),
        AgentTurn::decl(),
        AgentTraceSummary::decl(),
        AgentTrace::decl(),
        AgentTraceList::decl(),
        LlmSettings::decl(),
        LlmSettingsSnapshot::decl(),
        LlmSettingsRequest::decl(),
        LlmReasoningRequest::decl(),
        LlmReasoningResult::decl(),
        LlmTestResult::decl(),
        LlmModelsRequest::decl(),
        LlmModelOption::decl(),
        LlmModelsResult::decl(),
        ModelEnvironment::decl(),
        ModelRuntime::decl(),
        CatalogModel::decl(),
        InstalledModel::decl(),
        ModelDownload::decl(),
        ModelLibrarySnapshot::decl(),
        LauncherServiceId::decl(),
        LauncherServiceState::decl(),
        LauncherService::decl(),
        LauncherSetup::decl(),
        LauncherSnapshot::decl(),
        TrainingJob::decl(),
        TrainingPerformance::decl(),
        ModelVersion::decl(),
        TrainingSnapshot::decl(),
        TrainingModelRuntimeSnapshot::decl(),
        TrainingModelRuntimeRequest::decl(),
        TrainingClipMetadata::decl(),
        TrainingTextMode::decl(),
        TrainingCreateRequest::decl(),
        TrainingTranscribeRequest::decl(),
        TrainingTranscription::decl(),
        TrainingAuditionRequest::decl(),
        RuntimeMeasurement::decl(),
        RuntimePresetSnapshot::decl(),
        RuntimeMeasureRequest::decl(),
        VoiceProfile::decl(),
        CharacterMapping::decl(),
        CharacterProfile::decl(),
        ResourceSnapshot::decl(),
        VoiceCreateRequest::decl(),
        ResourceSelection::decl(),
        CharacterSaveRequest::decl(),
        CharacterPreviewRequest::decl(),
        VtsModel::decl(),
        ImportedModel::decl(),
        VtsHotkey::decl(),
        ObsOperation::decl(),
        ObsSnapshot::decl(),
        ObsSettingsSnapshot::decl(),
        ObsSettingsRequest::decl(),
        DesktopResourceOperation::decl(),
        DesktopResourceResult::decl(),
        LiveConnectionPhase::decl(),
        LiveConnectionSnapshot::decl(),
        LiveSettingsSnapshot::decl(),
        LiveSettingsRequest::decl(),
        AgentSettings::decl(),
        EventPayload::decl(),
        ViewerIdentityKind::decl(),
        ViewerIdentityInput::decl(),
        GiftMetadataInput::decl(),
        LiveEventInput::decl(),
        EventBatchRequest::decl(),
        EventBatchResult::decl(),
        AgentPhase::decl(),
        AgentEventStatus::decl(),
        AgentEventSnapshot::decl(),
        AgentSnapshot::decl(),
        AudioFormat::decl(),
        AudioChunk::decl(),
        SpeechRequest::decl(),
        SpeechStatus::decl(),
        SpeechSnapshot::decl(),
        ServerStatus::decl(),
        ServerHealth::decl(),
        ErrorResponse::decl(),
        ExecutionStatus::decl(),
        ExecutionReceipt::decl(),
        ServerCommand::decl(),
        ClientMessage::decl(),
    ];
    let mut output =
        String::from("// Generated by meowlive-protocol/export-types. Do not edit.\n\n");
    output.push_str(&format!(
        "export const PROTOCOL_VERSION = {} as const;\n\n",
        meowlive_protocol::PROTOCOL_VERSION
    ));
    for declaration in types {
        let declaration = declaration
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
        output.push_str(&format!("export {declaration}\n\n"));
    }
    if env::args().any(|arg| arg == "--check") {
        if fs::read_to_string(&path).ok().as_deref() != Some(&output) {
            eprintln!(
                "TypeScript contracts are stale; run cargo run -p meowlive-protocol --bin export-types"
            );
            return ExitCode::FAILURE;
        }
    } else if let Err(error) = fs::write(path, output) {
        eprintln!("Cannot write TypeScript contracts: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
