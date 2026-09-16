mod live_support;
use live_support::*;
use meowlive_application::ports::speech::{
    PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest,
};
use meowlive_protocol::live::LiveConnectionPhase as Phase;
use meowlive_server::{config::AppConfig, state::AppState};
use std::{sync::Arc, time::Duration};

struct Speech;
impl SpeechSynthesizer for Speech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async {
            Ok(PcmAudio {
                sample_rate: 24000,
                channels: 1,
                samples: vec![0],
            })
        })
    }
}

#[tokio::test]
async fn invalid_and_over_capacity_events_are_counted_without_losing_the_connection() {
    let (source, senders) = source(1, None);
    let mut config = AppConfig::default();
    config.live.enabled = true;
    config.live.app_id = 1;
    config.agent.pending_capacity = 1;
    let state = AppState::with_services(config, Arc::new(Speech), None, Some(source));
    state.connect_live().await.unwrap();
    phase(&state, Phase::Connected).await;
    senders[0].send(Ok(Some(event("first")))).await.unwrap();
    senders[0].send(Ok(Some(event("second")))).await.unwrap();
    senders[0].send(Ok(Some(event("")))).await.unwrap();
    senders[0].send(Ok(Some(event("first")))).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.live_snapshot().await.duplicate_events == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let snapshot = state.live_snapshot().await;
    assert_eq!(snapshot.phase, Phase::Connected);
    assert_eq!(snapshot.accepted_events, 1);
    assert_eq!(snapshot.rejected_events, 2);
    assert_eq!(snapshot.duplicate_events, 1);
    state.shutdown().await;
}

#[tokio::test]
async fn shutdown_closes_active_connection_and_rejects_new_connect_requests() {
    let (source, _senders) = source(1, None);
    let state = state(source.clone());
    state.connect_live().await.unwrap();
    phase(&state, Phase::Connected).await;
    state.shutdown().await;
    assert_eq!(state.live_snapshot().await.phase, Phase::Disconnected);
    assert_eq!(source.closed.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(state.connect_live().await.is_err());
}
