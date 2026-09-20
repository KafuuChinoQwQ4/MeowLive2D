mod support;

use meowlive_application::ports::{
    live_source::{LiveConnection, LiveFuture, LiveSource, LiveSourceError},
    llm::{AgentDecision, DecisionFuture, DecisionRequest, LanguageModel},
    speech::{PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
};
use meowlive_domain::event::LiveEvent;
use meowlive_protocol::live::LiveConnectionPhase as Phase;
use meowlive_server::{config::AppConfig, state::AppState, transport::http::router};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Mutex, Notify, mpsc};

type Item = Result<Option<LiveEvent>, LiveSourceError>;

struct PlannedConnection {
    room_id: String,
    rx: mpsc::Receiver<Item>,
    close_started: Arc<Notify>,
    close_gate: Option<Arc<Notify>>,
    close_error: Option<LiveSourceError>,
    closed: Arc<AtomicUsize>,
}

impl LiveConnection for PlannedConnection {
    fn room_id(&self) -> &str {
        &self.room_id
    }

    fn next(&mut self) -> LiveFuture<'_, Option<LiveEvent>> {
        Box::pin(async { self.rx.recv().await.unwrap_or(Ok(None)) })
    }

    fn close(&mut self) -> LiveFuture<'_, ()> {
        Box::pin(async {
            self.close_started.notify_one();
            if let Some(gate) = &self.close_gate {
                gate.notified().await;
            }
            self.closed.fetch_add(1, Ordering::SeqCst);
            match &self.close_error {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        })
    }
}

struct PlannedSource {
    sessions: Mutex<VecDeque<Result<Box<dyn LiveConnection>, LiveSourceError>>>,
    connects: AtomicUsize,
}

impl LiveSource for PlannedSource {
    fn connect(&self) -> LiveFuture<'_, Box<dyn LiveConnection>> {
        Box::pin(async move {
            self.connects.fetch_add(1, Ordering::SeqCst);
            self.sessions
                .lock()
                .await
                .pop_front()
                .unwrap_or_else(|| Err(LiveSourceError::new("no more sessions", false)))
        })
    }
}

struct Plan {
    room_id: &'static str,
    close_gate: Option<Arc<Notify>>,
    close_error: Option<LiveSourceError>,
}

struct Controls {
    senders: Vec<mpsc::Sender<Item>>,
    close_started: Vec<Arc<Notify>>,
    closed: Arc<AtomicUsize>,
}

fn planned_source(plans: Vec<Plan>) -> (Arc<PlannedSource>, Controls) {
    let closed = Arc::new(AtomicUsize::new(0));
    let mut sessions = VecDeque::new();
    let mut senders = Vec::new();
    let mut close_started = Vec::new();
    for plan in plans {
        let (tx, rx) = mpsc::channel(8);
        let started = Arc::new(Notify::new());
        senders.push(tx);
        close_started.push(started.clone());
        sessions.push_back(Ok(Box::new(PlannedConnection {
            room_id: plan.room_id.into(),
            rx,
            close_started: started,
            close_gate: plan.close_gate,
            close_error: plan.close_error,
            closed: closed.clone(),
        }) as Box<dyn LiveConnection>));
    }
    (
        Arc::new(PlannedSource {
            sessions: Mutex::new(sessions),
            connects: AtomicUsize::new(0),
        }),
        Controls {
            senders,
            close_started,
            closed,
        },
    )
}

struct Speech;
impl SpeechSynthesizer for Speech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async {
            Ok(PcmAudio {
                sample_rate: 24_000,
                channels: 1,
                samples: vec![0],
            })
        })
    }
}

struct Model;
impl LanguageModel for Model {
    fn decide(&self, _: DecisionRequest) -> DecisionFuture<'_> {
        Box::pin(async {
            Ok(AgentDecision {
                reply_to: vec![],
                text: None,
                topic: None,
            })
        })
    }
}

fn state(source: Arc<dyn LiveSource>, reconnect_ms: u64, model: bool) -> AppState {
    let mut config = AppConfig::default();
    config.viewers.enabled = false;
    config.live.enabled = true;
    config.live.app_id = 1;
    config.live.reconnect_initial_ms = reconnect_ms;
    config.live.reconnect_max_ms = reconnect_ms;
    AppState::with_services(
        config,
        Arc::new(Speech),
        model.then(|| Arc::new(Model) as Arc<dyn LanguageModel>),
        Some(source),
    )
}

async fn wait_for_phase(state: &AppState, expected: Phase) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.live_snapshot().await.phase != expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn receive_failure_transitions_before_delayed_close_and_keeps_single_owner() {
    let close_gate = Arc::new(Notify::new());
    let (source, controls) = planned_source(vec![
        Plan {
            room_id: "old-room",
            close_gate: Some(close_gate.clone()),
            close_error: None,
        },
        Plan {
            room_id: "new-room",
            close_gate: None,
            close_error: None,
        },
    ]);
    let state = state(source.clone(), 10, false);
    state.connect_live().await.unwrap();
    wait_for_phase(&state, Phase::Connected).await;

    controls.senders[0]
        .send(Err(LiveSourceError::new("transport lost", true)))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), controls.close_started[0].notified())
        .await
        .unwrap();

    let cleanup = state.live_snapshot().await;
    assert_eq!(cleanup.phase, Phase::Reconnecting);
    assert_eq!(cleanup.room_id, None);
    assert_eq!(
        state.connect_live().await.unwrap().phase,
        Phase::Reconnecting
    );
    assert_eq!(source.connects.load(Ordering::SeqCst), 1);

    close_gate.notify_one();
    wait_for_phase(&state, Phase::Connected).await;
    let reconnected = state.live_snapshot().await;
    assert_eq!(reconnected.room_id.as_deref(), Some("new-room"));
    assert_eq!(source.connects.load(Ordering::SeqCst), 2);
    state.shutdown().await;
}

#[tokio::test]
async fn permanent_failure_marks_cleanup_then_close_failure_stops_and_shutdown_finishes() {
    let close_gate = Arc::new(Notify::new());
    let (source, controls) = planned_source(vec![Plan {
        room_id: "room-1",
        close_gate: Some(close_gate.clone()),
        close_error: Some(LiveSourceError::new("remote close rejected", true)),
    }]);
    let state = state(source.clone(), 10, true);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(axum::serve(listener, router(state.clone())).into_future());
    let (control, audio, _) = support::pair(&base).await;
    support::await_connected(&state, true).await;
    state.resume_agent().await.unwrap();
    assert!(!state.agent_snapshot().await.paused);
    state.connect_live().await.unwrap();
    wait_for_phase(&state, Phase::Connected).await;

    controls.senders[0]
        .send(Err(LiveSourceError::new("authorization revoked", false)))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), controls.close_started[0].notified())
        .await
        .unwrap();

    let cleanup = state.live_snapshot().await;
    assert_eq!(cleanup.phase, Phase::Disconnecting);
    assert_eq!(cleanup.room_id, None);
    assert!(state.agent_snapshot().await.paused);
    assert!(state.connect_live().await.is_err());
    close_gate.notify_one();
    wait_for_phase(&state, Phase::Failed).await;
    let failed = state.live_snapshot().await;
    assert_eq!(failed.room_id, None);
    assert!(
        failed
            .last_error
            .as_deref()
            .unwrap()
            .contains("remote close rejected")
    );
    assert_eq!(source.connects.load(Ordering::SeqCst), 1);
    assert_eq!(controls.closed.load(Ordering::SeqCst), 1);
    tokio::time::timeout(Duration::from_secs(2), state.shutdown())
        .await
        .unwrap();
    drop(control);
    drop(audio);
    server.abort();
}

#[tokio::test]
async fn cancellation_during_backoff_does_not_reconnect_or_increment_attempts() {
    let (source, controls) = planned_source(vec![
        Plan {
            room_id: "room-1",
            close_gate: None,
            close_error: None,
        },
        Plan {
            room_id: "room-2",
            close_gate: None,
            close_error: None,
        },
    ]);
    let state = state(source.clone(), 5_000, false);
    state.connect_live().await.unwrap();
    wait_for_phase(&state, Phase::Connected).await;
    controls.senders[0]
        .send(Err(LiveSourceError::new("transport lost", true)))
        .await
        .unwrap();
    wait_for_phase(&state, Phase::Reconnecting).await;

    assert_eq!(state.disconnect_live().await.phase, Phase::Disconnecting);
    wait_for_phase(&state, Phase::Disconnected).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    let snapshot = state.live_snapshot().await;
    assert_eq!(snapshot.reconnect_attempts, 0);
    assert_eq!(snapshot.room_id, None);
    assert_eq!(source.connects.load(Ordering::SeqCst), 1);
    state.shutdown().await;
}
