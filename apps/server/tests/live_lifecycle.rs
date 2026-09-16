mod live_support;
use live_support::*;
use meowlive_application::ports::live_source::LiveSourceError;
use meowlive_protocol::live::LiveConnectionPhase as Phase;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::sync::Notify;

#[tokio::test]
async fn one_connection_admits_events_once_and_disconnect_prevents_late_admission() {
    let (source, senders) = source(1, None);
    let state = state(source.clone());
    let (a, b) = tokio::join!(state.connect_live(), state.connect_live());
    assert!(a.is_ok() && b.is_ok());
    phase(&state, Phase::Connected).await;
    assert_eq!(source.connects.load(Ordering::SeqCst), 1);
    senders[0]
        .send(Ok(Some(event("bilibili:123:one"))))
        .await
        .unwrap();
    senders[0]
        .send(Ok(Some(event("bilibili:123:one"))))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.live_snapshot().await.duplicate_events != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(state.live_snapshot().await.accepted_events, 1);
    assert_eq!(state.agent_snapshot().await.events.len(), 1);
    assert_eq!(state.disconnect_live().await.room_id, None);
    let _ = senders[0].send(Ok(Some(event("late")))).await;
    phase(&state, Phase::Disconnected).await;
    let snapshot = state.live_snapshot().await;
    assert_eq!(snapshot.room_id, None);
    assert_eq!(snapshot.accepted_events, 1);
    assert_eq!(source.closed.load(Ordering::SeqCst), 1);
    state.shutdown().await;
}

#[tokio::test]
async fn cancellation_waits_for_connect_result_and_closes_it_without_receiving_events() {
    let gate = Arc::new(Notify::new());
    let (source, _senders) = source(1, Some(gate.clone()));
    let state = state(source.clone());
    state.connect_live().await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while source.connects.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(state.disconnect_live().await.phase, Phase::Disconnecting);
    assert!(state.connect_live().await.is_err());
    gate.notify_one();
    phase(&state, Phase::Disconnected).await;
    assert_eq!(source.closed.load(Ordering::SeqCst), 1);
    assert_eq!(state.live_snapshot().await.accepted_events, 0);
    state.shutdown().await;
}

#[tokio::test]
async fn transient_failure_reconnects_but_permanent_failure_stops() {
    let (source, senders) = source(2, None);
    let state = state(source.clone());
    state.connect_live().await.unwrap();
    phase(&state, Phase::Connected).await;
    senders[0]
        .send(Err(LiveSourceError::new("transport lost", true)))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while source.connects.load(Ordering::SeqCst) != 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    phase(&state, Phase::Connected).await;
    assert_eq!(state.live_snapshot().await.reconnect_attempts, 1);
    senders[1]
        .send(Err(LiveSourceError::new("authorization revoked", false)))
        .await
        .unwrap();
    phase(&state, Phase::Failed).await;
    assert!(state.agent_snapshot().await.paused);
    assert_eq!(source.closed.load(Ordering::SeqCst), 2);
    state.shutdown().await;
}
