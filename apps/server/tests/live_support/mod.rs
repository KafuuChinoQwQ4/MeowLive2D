#![allow(dead_code)]
use meowlive_application::ports::{
    live_source::{LiveConnection, LiveFuture, LiveSource, LiveSourceError},
    speech::{PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
};
use meowlive_domain::event::{EventKind, LiveEvent};
use meowlive_protocol::live::LiveConnectionPhase;
use meowlive_server::{config::AppConfig, state::AppState};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Mutex, Notify, mpsc};

pub type Item = Result<Option<LiveEvent>, LiveSourceError>;
pub struct ControlledSource {
    pub sessions: Mutex<VecDeque<Result<Box<dyn LiveConnection>, LiveSourceError>>>,
    pub connects: AtomicUsize,
    pub closed: Arc<AtomicUsize>,
    pub gate: Option<Arc<Notify>>,
}
impl LiveSource for ControlledSource {
    fn connect(&self) -> LiveFuture<'_, Box<dyn LiveConnection>> {
        Box::pin(async move {
            self.connects.fetch_add(1, Ordering::SeqCst);
            if let Some(gate) = &self.gate {
                gate.notified().await;
            }
            self.sessions
                .lock()
                .await
                .pop_front()
                .unwrap_or_else(|| Err(LiveSourceError::new("no more sessions", false)))
        })
    }
}
struct Connection {
    rx: mpsc::Receiver<Item>,
    closed: Arc<AtomicUsize>,
}
impl LiveConnection for Connection {
    fn room_id(&self) -> &str {
        "123"
    }
    fn next(&mut self) -> LiveFuture<'_, Option<LiveEvent>> {
        Box::pin(async { self.rx.recv().await.unwrap_or(Ok(None)) })
    }
    fn close(&mut self) -> LiveFuture<'_, ()> {
        Box::pin(async {
            self.closed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}
pub fn source(
    count: usize,
    gate: Option<Arc<Notify>>,
) -> (Arc<ControlledSource>, Vec<mpsc::Sender<Item>>) {
    let closed = Arc::new(AtomicUsize::new(0));
    let mut sessions = VecDeque::new();
    let mut senders = vec![];
    for _ in 0..count {
        let (tx, rx) = mpsc::channel(32);
        senders.push(tx);
        sessions.push_back(Ok(Box::new(Connection {
            rx,
            closed: closed.clone(),
        }) as Box<dyn LiveConnection>));
    }
    (
        Arc::new(ControlledSource {
            sessions: Mutex::new(sessions),
            connects: AtomicUsize::new(0),
            closed,
            gate,
        }),
        senders,
    )
}
struct Speech;
impl SpeechSynthesizer for Speech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async {
            Ok(PcmAudio {
                sample_rate: 24000,
                channels: 1,
                samples: vec![0; 240],
            })
        })
    }
}
pub fn state(source: Arc<dyn LiveSource>) -> AppState {
    let mut config = AppConfig::default();
    config.viewers.enabled = false;
    config.live.enabled = true;
    config.live.app_id = 1;
    config.live.reconnect_initial_ms = 10;
    config.live.reconnect_max_ms = 20;
    AppState::with_services(config, Arc::new(Speech), None, Some(source))
}
pub fn event(id: &str) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "bilibili".into(),
        viewer: "观众".into(),
        viewer_identity: None,
        occurred_at_ms: 1_780_000_000_000,
        gift_metadata: None,
        kind: EventKind::Gift {
            name: "小花花".into(),
            count: 2,
        },
    }
}
pub async fn phase(state: &AppState, expected: LiveConnectionPhase) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.live_snapshot().await.phase != expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
