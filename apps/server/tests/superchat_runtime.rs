mod live_support;
mod support;

use meowlive_application::ports::{
    llm::{AgentDecision, DecisionFuture, DecisionRequest, LanguageModel},
    speech::{PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
};
use meowlive_domain::event::{EventKind, LiveEvent};
use meowlive_protocol::{agent::AgentEventStatus, live::LiveConnectionPhase};
use meowlive_server::{
    agent::run_agent, config::AppConfig, state::AppState, transport::http::router, viewers::utc_ms,
    worker::run_worker,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, mpsc},
    task::JoinHandle,
};

struct RecordingModel(Arc<Mutex<Vec<DecisionRequest>>>);
impl LanguageModel for RecordingModel {
    fn decide(&self, request: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async move {
            let reply_to = request
                .events
                .iter()
                .map(|event| event.id.clone())
                .collect();
            self.0.lock().await.push(request);
            Ok(AgentDecision {
                reply_to,
                text: Some("答".repeat(500)),
                topic: None,
            })
        })
    }
}

struct RecordingSpeech(Arc<Mutex<Vec<SynthesisRequest>>>);
impl SpeechSynthesizer for RecordingSpeech {
    fn synthesize(&self, request: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async move {
            self.0.lock().await.push(request);
            Ok(PcmAudio {
                sample_rate: 24000,
                channels: 1,
                samples: vec![1000; 240],
            })
        })
    }
}

struct Harness {
    state: AppState,
    base: String,
    source: mpsc::Sender<live_support::Item>,
    decisions: Arc<Mutex<Vec<DecisionRequest>>>,
    synthesis: Arc<Mutex<Vec<SynthesisRequest>>>,
    tasks: Vec<JoinHandle<()>>,
}

impl Harness {
    async fn start() -> Self {
        let (source, senders) = live_support::source(1, None);
        let mut config = AppConfig::default();
        config.viewers.enabled = false;
        config.live.enabled = true;
        config.live.app_id = 1;
        config.agent.event_ttl_ms = 1000;
        config.agent.proactive_enabled = false;
        config.llm.max_retries = 0;
        let decisions = Arc::new(Mutex::new(Vec::new()));
        let synthesis = Arc::new(Mutex::new(Vec::new()));
        let state = AppState::with_services(
            config,
            Arc::new(RecordingSpeech(synthesis.clone())),
            Some(Arc::new(RecordingModel(decisions.clone()))),
            Some(source),
        );
        state.resources.select_voice("default").unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("ws://{}", listener.local_addr().unwrap());
        let app = router(state.clone());
        let tasks = vec![
            tokio::spawn(async move { axum::serve(listener, app).await.unwrap() }),
            tokio::spawn(run_worker(state.clone())),
            tokio::spawn(run_agent(state.clone())),
        ];
        state.connect_live().await.unwrap();
        live_support::phase(&state, LiveConnectionPhase::Connected).await;
        Self {
            state,
            base,
            source: senders[0].clone(),
            decisions,
            synthesis,
            tasks,
        }
    }

    async fn send(&self, event: LiveEvent) {
        self.source.send(Ok(Some(event))).await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while self.state.live_snapshot().await.accepted_events == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

fn super_chat(id: &str, start_at_ms: u64, end_at_ms: u64) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "bilibili".into(),
        viewer: "观众甲".into(),
        viewer_identity: None,
        // A fresh notification must not reset an old SC's display period.
        occurred_at_ms: utc_ms(),
        gift_metadata: None,
        kind: EventKind::SuperChat {
            text: "猫".repeat(500),
            amount_cny: 198,
            start_at_ms,
            end_at_ms,
        },
    }
}

#[tokio::test]
async fn live_super_chat_older_than_chat_ttl_preserves_amount_and_complete_speech() {
    let harness = Harness::start().await;
    let (mut control, mut audio, _) = support::pair(&harness.base).await;
    support::await_connected(&harness.state, true).await;
    let now = utc_ms();
    let start_at_ms = now - 120_000;
    let end_at_ms = now + 120_000;
    let mut event = super_chat("sc-long-lived", start_at_ms, end_at_ms);
    event.occurred_at_ms = start_at_ms;
    harness.send(event).await;
    let snapshot = harness.state.agent_snapshot().await;
    assert_eq!(snapshot.events[0].status, AgentEventStatus::Pending);
    assert_eq!(harness.state.live_snapshot().await.rejected_events, 0);
    harness.state.resume_agent().await.unwrap();

    let speak = support::next_json(&mut control).await;
    assert_eq!(speak["type"], "speak");
    support::next_data(&mut audio).await;
    let decisions = harness.decisions.lock().await;
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].events.len(), 1);
    assert_eq!(decisions[0].events[0].id, "sc-long-lived");
    assert_eq!(
        decisions[0].events[0].kind,
        EventKind::SuperChat {
            text: "猫".repeat(500),
            amount_cny: 198,
            start_at_ms,
            end_at_ms,
        }
    );
    drop(decisions);
    let expected = format!(
        "感谢观众甲的198元SC。留言说：{}。{}",
        "猫".repeat(500),
        "答".repeat(500)
    );
    let synthesis = harness.synthesis.lock().await;
    assert_eq!(synthesis.len(), 1);
    assert_eq!(synthesis[0].text, expected);
    drop(synthesis);
    let queued = harness.state.snapshot().await;
    assert_eq!(queued.speeches.len(), 1);
    assert_eq!(queued.speeches[0].text, expected);
    let snapshot = harness.state.agent_snapshot().await;
    assert_eq!(snapshot.events[0].status, AgentEventStatus::Ready);
    assert_eq!(
        snapshot.events[0].speech_id.as_deref(),
        Some(queued.speeches[0].id.as_str())
    );
    harness.state.shutdown().await;
}

#[tokio::test]
async fn expired_live_super_chat_with_fresh_notice_timestamp_never_reaches_model_or_speech() {
    let harness = Harness::start().await;
    let (_control, _audio, _) = support::pair(&harness.base).await;
    support::await_connected(&harness.state, true).await;
    let now = utc_ms();
    harness
        .send(super_chat("sc-expired", now - 120_000, now - 60_000))
        .await;
    harness.state.resume_agent().await.unwrap();
    let snapshot = harness.state.agent_snapshot().await;
    assert_eq!(snapshot.events[0].status, AgentEventStatus::Expired);
    assert_eq!(snapshot.events[0].speech_id, None);
    // Cross several worker polling cycles while the bridge remains available.
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(harness.decisions.lock().await.is_empty());
    assert!(harness.synthesis.lock().await.is_empty());
    assert!(harness.state.snapshot().await.speeches.is_empty());
    assert_eq!(
        harness.state.agent_snapshot().await.events[0].status,
        AgentEventStatus::Expired
    );
    harness.state.shutdown().await;
}

#[tokio::test]
async fn delayed_live_room_entry_is_not_welcomed_as_a_new_arrival() {
    let harness = Harness::start().await;
    let (_control, _audio, _) = support::pair(&harness.base).await;
    support::await_connected(&harness.state, true).await;
    let now = utc_ms();
    let mut event = super_chat("late-entry", now - 20_000, now);
    event.kind = EventKind::RoomEnter;
    event.occurred_at_ms = now - 20_000;
    harness.send(event).await;
    assert_eq!(
        harness.state.agent_snapshot().await.events[0].status,
        AgentEventStatus::Expired
    );
    harness.state.resume_agent().await.unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(harness.decisions.lock().await.is_empty());
    assert!(harness.synthesis.lock().await.is_empty());
    harness.state.shutdown().await;
}
