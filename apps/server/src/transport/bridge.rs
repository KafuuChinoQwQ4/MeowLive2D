//! 执行端控制与音频的独立发送循环，连接失效时撤销整对连接。
use super::mapping;
use crate::state::AppState;
use axum::extract::ws::{Message, WebSocket};
use meowlive_protocol::{
    PROTOCOL_VERSION,
    audio::AudioChunk,
    control::{ClientMessage, ServerCommand},
};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

async fn send(socket: &mut WebSocket, message: Message, cancel: &CancellationToken) -> bool {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => false,
        result = tokio::time::timeout(Duration::from_secs(5), socket.send(message)) => matches!(result, Ok(Ok(()))),
    }
}

pub async fn control(
    mut socket: WebSocket,
    state: AppState,
    id: String,
    mut outgoing: mpsc::Receiver<ServerCommand>,
    cancel: CancellationToken,
) {
    let hello = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    let valid = match hello {
        Ok(Some(Ok(Message::Text(text)))) => matches!(
            serde_json::from_str::<ClientMessage>(&text),
            Ok(ClientMessage::Hello {
                protocol_version: PROTOCOL_VERSION
            })
        ),
        _ => false,
    };
    if !valid {
        state.disconnect(&id).await;
        return;
    }
    let command = ServerCommand::Hello {
        protocol_version: PROTOCOL_VERSION,
        session_id: state.session_id.to_string(),
        bridge_id: id.clone(),
        generation: state.inner.lock().await.queue.generation(),
    };
    if !send(
        &mut socket,
        Message::Text(
            serde_json::to_string(&command)
                .expect("serializable command")
                .into(),
        ),
        &cancel,
    )
    .await
    {
        state.disconnect(&id).await;
        return;
    }
    let pair_deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(pair_deadline);
    let mut pair_checked = false;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    let mut last_received = tokio::time::Instant::now();
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            _ = &mut pair_deadline, if !pair_checked => {
                pair_checked = true;
                if !state.inner.lock().await.queue.is_connected() { break; }
            }
            command = outgoing.recv() => {
                let Some(command) = command else { break; };
                if let ServerCommand::Speak { generation, .. } = &command {
                    if *generation != state.inner.lock().await.queue.generation() { continue; }
                }
                if let ServerCommand::Resource {request_id,..}=&command {
                    if !state.resource_pending.lock().expect("resource requests lock").contains_key(request_id){continue;}
                }
                if !send(&mut socket, Message::Text(serde_json::to_string(&command).expect("serializable command").into()), &cancel).await { break; }
            }
            frame = socket.recv() => {
                last_received = tokio::time::Instant::now();
                match frame {
                    Some(Ok(Message::Text(text))) => {
                        let receipt = match serde_json::from_str(&text) {
                            Ok(ClientMessage::Receipt { receipt }) => receipt,
                            Ok(ClientMessage::ResourceResult { request_id, result }) => {
                                state.resource_reply(&id, request_id, result);
                                continue;
                            }
                            _ => break,
                        };
                        let mut inner = state.inner.lock().await;
                        if inner.bridge.as_ref().is_none_or(|b| b.id != id) { break; }
                        // Device errors are user-facing summaries, never arbitrary executable data.
                        let error = receipt.error.map(|s| s.chars().take(256).collect());
                        inner.queue.apply_receipt(&receipt.utterance_id, receipt.generation, mapping::receipt(receipt.status), error);
                        // Commit the Agent result with the device receipt, before
                        // a later Stop can prune the speech queue's short history.
                        state.sync_agent(&mut inner);
                        state.wake.notify_one();
                        state.agent_wake.notify_one();
                    }
                    Some(Ok(Message::Ping(bytes))) => { if !send(&mut socket, Message::Pong(bytes), &cancel).await { break; } }
                    Some(Ok(Message::Pong(_))) => {}
                    _ => break,
                }
            }
            _ = heartbeat.tick() => {
                if last_received.elapsed() > Duration::from_secs(30) { break; }
                if !send(&mut socket, Message::Ping(Vec::new().into()), &cancel).await { break; }
            }
        }
    }
    state.disconnect(&id).await;
}

pub async fn audio(
    mut socket: WebSocket,
    state: AppState,
    id: String,
    mut outgoing: mpsc::Receiver<AudioChunk>,
    cancel: CancellationToken,
) {
    {
        let mut inner = state.inner.lock().await;
        if inner.bridge.as_ref().is_none_or(|b| b.id != id) {
            return;
        }
        inner.queue.set_connected(true);
    }
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    let mut last_received = tokio::time::Instant::now();
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            frame = socket.recv() => {
                last_received = tokio::time::Instant::now();
                match frame {
                    Some(Ok(Message::Ping(bytes))) => { if !send(&mut socket, Message::Pong(bytes), &cancel).await { break; } }
                    Some(Ok(Message::Pong(_))) => {}
                    _ => break,
                }
            }
            chunk = outgoing.recv() => {
                let Some(chunk) = chunk else { break; };
                if chunk.generation != state.inner.lock().await.queue.generation() { continue; }
                let Ok(bytes) = chunk.encode() else { break; };
                if !send(&mut socket, Message::Binary(bytes.into()), &cancel).await { break; }
            }
            _ = heartbeat.tick() => {
                if last_received.elapsed() > Duration::from_secs(30) { break; }
                if !send(&mut socket, Message::Ping(Vec::new().into()), &cancel).await { break; }
            }
        }
    }
    state.disconnect(&id).await;
}
