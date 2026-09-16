//! 有界业务队列与桥接连接的进程内所有权，锁内不进行网络或模型等待。
use crate::{config::AppConfig, transport::mapping};
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
    pub session_id: Arc<String>,
    pub(crate) inner: Arc<Mutex<Inner>>,
    pub(crate) wake: Arc<Notify>,
    pub(crate) synthesizer: Arc<dyn SpeechSynthesizer>,
    pub(crate) model: Option<Arc<dyn LanguageModel>>,
    pub(crate) agent_wake: Arc<Notify>,
    pub(crate) started: tokio::time::Instant,
    pub(crate) live_source: Option<Arc<dyn LiveSource>>,
    pub(crate) stopping: CancellationToken,
    pub training: Arc<meowlive_application::training::TrainingManager>,
    pub(crate) model_synthesizer:
        Option<Arc<meowlive_adapters::speech::model_synthesizer::ModelSynthesizer>>,
    pub(crate) measurement: Arc<Mutex<Option<meowlive_protocol::training::RuntimeMeasurement>>>,
    pub(crate) gpu_busy: Arc<std::sync::atomic::AtomicBool>,
    pub resources: Arc<meowlive_application::resources::ResourceLibrary>,
    pub(crate) resource_edits: Arc<Mutex<()>>,
    pub(crate) resource_requests: Arc<tokio::sync::Semaphore>,
    pub(crate) resource_pending: Arc<std::sync::Mutex<crate::resources::PendingRequests>>,
    pub(crate) resource_changing: Arc<std::sync::atomic::AtomicBool>,
}

pub(crate) struct Inner {
    pub queue: SpeechQueue,
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
        let agent = AgentSession::new(config.agent.settings(), config.agent.limits())
            .expect("validated Agent configuration");
        let live = crate::live::LiveState::new(&config.live, live_source.is_some());
        Self {
            config: Arc::new(config),
            session_id: Arc::new(uuid::Uuid::new_v4().to_string()),
            inner: Arc::new(Mutex::new(Inner {
                queue,
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
            live_source,
            stopping: CancellationToken::new(),
            training: Arc::new(meowlive_application::training::TrainingManager::disabled()),
            model_synthesizer: None,
            measurement: Arc::new(Mutex::new(None)),
            gpu_busy: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            resources: Arc::new(meowlive_application::resources::ResourceLibrary::memory()),
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
