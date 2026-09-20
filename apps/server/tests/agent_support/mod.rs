#![allow(dead_code)]
use meowlive_application::ports::{
    llm::{AgentDecision, DecisionFuture, DecisionRequest, LanguageModel},
    speech::{PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
};
use meowlive_protocol::agent::{EventPayload, LiveEventInput};
use meowlive_server::{
    agent::run_agent, config::AppConfig, state::AppState, transport::http::router,
    worker::run_worker,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::task::JoinHandle;

pub struct ReplyModel {
    pub calls: Arc<AtomicUsize>,
}
impl LanguageModel for ReplyModel {
    fn decide(&self, request: DecisionRequest) -> DecisionFuture<'_> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            Ok(AgentDecision {
                reply_to: request.events.iter().map(|e| e.id.clone()).collect(),
                text: Some("你好，欢迎来到直播间。".into()),
                topic: Some("打招呼".into()),
            })
        })
    }
}

struct FixedSpeech(Arc<AtomicUsize>);
impl SpeechSynthesizer for FixedSpeech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(PcmAudio {
                sample_rate: 24000,
                channels: 1,
                samples: vec![1000; 2400],
            })
        })
    }
}

pub struct Harness {
    pub state: AppState,
    pub base: String,
    pub tasks: Vec<JoinHandle<()>>,
    pub synthesis_calls: Arc<AtomicUsize>,
}
impl Harness {
    pub async fn new(model: Arc<dyn LanguageModel>) -> Self {
        let mut config = AppConfig::default();
        config.agent.cooldown_ms = 1000;
        Self::configured(config, model).await
    }
    pub async fn configured(mut config: AppConfig, model: Arc<dyn LanguageModel>) -> Self {
        config.viewers.enabled = false;
        let synthesis_calls = Arc::new(AtomicUsize::new(0));
        let state = AppState::with_model(
            config,
            Arc::new(FixedSpeech(synthesis_calls.clone())),
            Some(model),
        );
        // The harness explicitly chooses its fake voice; new installations start empty.
        state.resources.select_voice("default").unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("ws://{}", listener.local_addr().unwrap());
        let app = router(state.clone());
        let server = tokio::spawn(async {
            axum::serve(listener, app).await.unwrap();
        });
        let tasks = vec![
            server,
            tokio::spawn(run_worker(state.clone())),
            tokio::spawn(run_agent(state.clone())),
        ];
        Self {
            state,
            base,
            tasks,
            synthesis_calls,
        }
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

pub fn event(id: &str) -> LiveEventInput {
    LiveEventInput {
        id: id.into(),
        source: "simulator".into(),
        viewer: "观众".into(),
        viewer_identity: None,
        gift_metadata: None,
        kind: EventPayload::Chat {
            text: "你好".into(),
        },
    }
}
