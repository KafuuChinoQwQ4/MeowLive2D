//! 有界业务队列与桥接连接的进程内所有权，锁内不进行网络或模型等待。
use crate::{auth::AdminAuth, config::AppConfig, transport::mapping};
use meowlive_application::{
    agent::AgentSession,
    ports::{live_source::LiveSource, llm::LanguageModel, speech::SpeechSynthesizer},
    speech::SpeechQueue,
};
use meowlive_protocol::{
    PROTOCOL_VERSION,
    audio::AudioChunk,
    control::{ServerCommand, ServerStatus},
};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, mpsc};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub auth: Arc<AdminAuth>,
    pub llm_settings: Arc<crate::llm_settings::LlmSettingsStore>,
    pub live_settings: Arc<crate::live_settings::LiveSettingsStore>,
    pub agent_settings: Arc<crate::agent_settings::AgentSettingsStore>,
    pub session_id: Arc<String>,
    pub(crate) inner: Arc<Mutex<Inner>>,
    pub(crate) wake: Arc<Notify>,
    pub(crate) synthesizer: Arc<dyn SpeechSynthesizer>,
    pub(crate) model: Option<Arc<dyn LanguageModel>>,
    pub(crate) agent_wake: Arc<Notify>,
    pub(crate) started: tokio::time::Instant,
    pub(crate) stopping: CancellationToken,
    pub training: Arc<meowlive_application::training::TrainingManager>,
    pub(crate) model_synthesizer:
        Option<Arc<meowlive_adapters::speech::model_synthesizer::ModelSynthesizer>>,
    pub(crate) measurement: Arc<Mutex<Option<meowlive_protocol::training::RuntimeMeasurement>>>,
    pub(crate) gpu_busy: Arc<std::sync::atomic::AtomicBool>,
    pub resources: Arc<meowlive_application::resources::ResourceLibrary>,
    pub receipt_journal:
        Option<Arc<dyn meowlive_application::ports::receipt_journal::ReceiptJournal>>,
    pub viewer_merge_store:
        Option<Arc<dyn meowlive_application::ports::viewer_merge::ViewerMergeStore>>,
    pub relationship_store:
        Option<Arc<dyn meowlive_application::ports::relationships::RelationshipStore>>,
    pub(crate) relationship_graph: Arc<
        tokio::sync::RwLock<
            Option<Arc<dyn meowlive_application::ports::relationships::RelationshipGraph>>,
        >,
    >,
    pub memory_store: Option<Arc<dyn meowlive_application::ports::memory_store::MemoryStore>>,
    pub(crate) memory_extractor:
        Option<Arc<dyn meowlive_application::ports::memory::MemoryExtractor>>,
    pub(crate) memory_embedder:
        Option<Arc<dyn meowlive_application::ports::memory::MemoryEmbedder>>,
    pub(crate) knowledge_gate: Arc<tokio::sync::RwLock<()>>,
    pub(crate) knowledge_epoch: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) knowledge_deadline: Arc<std::sync::atomic::AtomicI64>,
    pub companionship_store:
        Option<Arc<dyn meowlive_application::ports::companionship::CompanionshipStore>>,
    pub(crate) receipts_pending: Arc<std::sync::atomic::AtomicU32>,
    pub(crate) receipt_problems: Arc<Mutex<Vec<meowlive_protocol::companionship::ReceiptProblem>>>,
    pub(crate) receipt_failures: Arc<std::sync::atomic::AtomicU64>,
    pub viewer_store: Option<Arc<dyn meowlive_application::ports::viewers::ViewerEventStore>>,
    pub(crate) viewer_requests: Arc<tokio::sync::Semaphore>,
    pub(crate) viewer_gaps: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) resource_edits: Arc<Mutex<()>>,
    pub(crate) resource_requests: Arc<tokio::sync::Semaphore>,
    pub(crate) resource_pending: Arc<std::sync::Mutex<crate::resources::PendingRequests>>,
    pub(crate) resource_changing: Arc<std::sync::atomic::AtomicBool>,
}

pub(crate) struct Inner {
    pub queue: SpeechQueue,
    pub knowledge: std::collections::HashMap<String, crate::memory::KnowledgeStamp>,
    pub bridge: Option<Bridge>,
    pub generation_cancel: CancellationToken,
    pub agent: AgentSession,
    pub agent_cancel: CancellationToken,
    pub live: crate::live::LiveState,
}

pub(crate) struct Bridge {
    pub id: String,
    pub control: mpsc::Sender<ServerCommand>,
    pub audio: Option<mpsc::Sender<AudioChunk>>,
    pub cancel: CancellationToken,
}

impl AppState {
    pub fn new(config: AppConfig, synthesizer: Arc<dyn SpeechSynthesizer>) -> Self {
        Self::with_model(config, synthesizer, None)
    }

    pub fn with_model(
        config: AppConfig,
        synthesizer: Arc<dyn SpeechSynthesizer>,
        model: Option<Arc<dyn LanguageModel>>,
    ) -> Self {
        Self::with_services(config, synthesizer, model, None)
    }

    pub fn with_services(
        config: AppConfig,
        synthesizer: Arc<dyn SpeechSynthesizer>,
        model: Option<Arc<dyn LanguageModel>>,
        live_source: Option<Arc<dyn LiveSource>>,
    ) -> Self {
        let queue = SpeechQueue::new(config.server.queue_capacity, config.server.history_limit);
        let auth = if config.auth.enabled {
            AdminAuth::locked(config.auth.session_lifetime_seconds)
        } else {
            AdminAuth::disabled()
        };
        let agent = AgentSession::new(config.agent.settings(), config.agent.limits())
            .expect("validated Agent configuration");
        let live = crate::live::LiveState::new(&config.live, live_source);
        Self {
            live_settings: Arc::new(crate::live_settings::LiveSettingsStore::default()),
            agent_settings: Arc::new(crate::agent_settings::AgentSettingsStore::default()),
            llm_settings: Arc::new(crate::llm_settings::LlmSettingsStore::new(
                config.llm.clone(),
                None,
            )),
            config: Arc::new(config),
            auth: Arc::new(auth),
            session_id: Arc::new(uuid::Uuid::new_v4().to_string()),
            inner: Arc::new(Mutex::new(Inner {
                queue,
                knowledge: std::collections::HashMap::new(),
                bridge: None,
                generation_cancel: CancellationToken::new(),
                agent,
                agent_cancel: CancellationToken::new(),
                live,
            })),
            wake: Arc::new(Notify::new()),
            synthesizer,
            model,
            agent_wake: Arc::new(Notify::new()),
            started: tokio::time::Instant::now(),
            stopping: CancellationToken::new(),
            training: Arc::new(meowlive_application::training::TrainingManager::disabled()),
            model_synthesizer: None,
            measurement: Arc::new(Mutex::new(None)),
            gpu_busy: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            resources: Arc::new(meowlive_application::resources::ResourceLibrary::memory()),
            memory_store: None,
            relationship_store: None,
            viewer_merge_store: None,
            receipt_journal: None,
            relationship_graph: Arc::new(tokio::sync::RwLock::new(None)),
            memory_extractor: None,
            memory_embedder: None,
            knowledge_gate: Arc::new(tokio::sync::RwLock::new(())),
            knowledge_epoch: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            knowledge_deadline: Arc::new(std::sync::atomic::AtomicI64::new(i64::MAX)),
            companionship_store: None,
            receipts_pending: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            receipt_problems: Arc::new(Mutex::new(Vec::new())),
            receipt_failures: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            viewer_store: None,
            viewer_requests: Arc::new(tokio::sync::Semaphore::new(8)),
            viewer_gaps: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            resource_edits: Arc::new(Mutex::new(())),
            resource_requests: Arc::new(tokio::sync::Semaphore::new(1)),
            resource_pending: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            resource_changing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn snapshot(&self) -> ServerStatus {
        let inner = self.inner.lock().await;
        ServerStatus {
            protocol_version: PROTOCOL_VERSION,
            session_id: self.session_id.to_string(),
            bridge_connected: inner.queue.is_connected(),
            generation: inner.queue.generation(),
            speeches: inner.queue.tasks().map(mapping::speech).collect(),
        }
    }

    pub(crate) async fn disconnect(&self, bridge_id: &str) {
        let mut inner = self.inner.lock().await;
        if inner
            .bridge
            .as_ref()
            .is_some_and(|bridge| bridge.id == bridge_id)
        {
            self.sync_agent(&mut inner);
            let pending = self.agent_speech(&mut inner);
            if let Some(bridge) = inner.bridge.take() {
                bridge.cancel.cancel();
            }
            inner.queue.disconnect();
            self.end_agent_speech(&mut inner, pending, true);
            inner.agent.stop(self.now_ms());
            inner.agent_cancel.cancel();
            inner.agent_cancel = CancellationToken::new();
            inner.generation_cancel.cancel();
            inner.generation_cancel = CancellationToken::new();
            self.wake.notify_one();
            self.agent_wake.notify_one();
        }
    }

    pub async fn stop(&self) -> ServerStatus {
        let mut inner = self.inner.lock().await;
        self.sync_agent(&mut inner);
        let pending = self.agent_speech(&mut inner);
        let generation = inner.queue.stop();
        self.end_agent_speech(&mut inner, pending, false);
        inner.agent.stop(self.now_ms());
        inner.agent_cancel.cancel();
        inner.agent_cancel = CancellationToken::new();
        inner.generation_cancel.cancel();
        inner.generation_cancel = CancellationToken::new();
        let disconnected = inner.bridge.as_ref().is_some_and(|bridge| {
            bridge
                .control
                .try_send(ServerCommand::Stop { generation })
                .is_err()
        });
        if disconnected {
            if let Some(bridge) = inner.bridge.take() {
                bridge.cancel.cancel();
            }
            inner.queue.disconnect();
        }
        self.wake.notify_one();
        self.agent_wake.notify_one();
        drop(inner);
        self.snapshot().await
    }

    pub async fn shutdown(&self) {
        self.stopping.cancel();
        self.disconnect_live().await;
        self.pause_agent().await;
        let id = self
            .inner
            .lock()
            .await
            .bridge
            .as_ref()
            .map(|b| b.id.clone());
        if let Some(id) = id {
            self.disconnect(&id).await;
        }
        self.wait_live_shutdown().await;
        let training = self.training.clone();
        let _ = tokio::task::spawn_blocking(move || training.shutdown()).await;
    }
}
