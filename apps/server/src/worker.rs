//! 将业务队列的当前任务交给语音 port，再通过有界桥接通道传输。
use crate::state::AppState;
use meowlive_application::ports::speech::SynthesisRequest;
use meowlive_domain::speech::SpeechTask;
use meowlive_protocol::{
    audio::{AudioChunk, AudioFormat},
    control::ServerCommand,
};
use std::time::Duration;

pub async fn run_worker(state: AppState) {
    loop {
        let notified = state.wake.notified();
        let work = {
            let mut inner = state.inner.lock().await;
            inner
                .queue
                .next_for_synthesis()
                .map(|task| (task, inner.generation_cancel.clone()))
        };
        let Some((task, cancel)) = work else {
            notified.await;
            continue;
        };
        let request = SynthesisRequest {
            text: task.text.as_str().to_owned(),
            voice_id: task.voice_id.as_str().to_owned(),
        };
        let generated = if state.synthesizer.owns_synthesis_lifetime() {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => continue,
                result = state.synthesizer.synthesize(request) => Ok(result),
            }
        } else {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => continue,
                result = tokio::time::timeout(Duration::from_secs(state.config.speech.timeout_seconds), state.synthesizer.synthesize(request)) => result,
            }
        };
        let audio = match generated {
            Ok(Ok(audio)) => audio,
            other => {
                let error = match other {
                    Ok(Err(error)) => error.message,
                    _ => "语音合成超时".into(),
                };
                state
                    .inner
                    .lock()
                    .await
                    .queue
                    .fail(&task.id, task.generation, error);
                continue;
            }
        };
        let format = AudioFormat {
            sample_rate: audio.sample_rate,
            channels: audio.channels,
        };
        if format.validate().is_err()
            || audio.samples.is_empty()
            || audio.samples.len() % usize::from(audio.channels.max(1)) != 0
            || audio.samples.len() > state.config.speech.max_audio_bytes / 2
        {
            state.inner.lock().await.queue.fail(
                &task.id,
                task.generation,
                "语音合成结果格式或大小无效",
            );
            continue;
        }
        let knowledge_gate = state.knowledge_gate.read().await;
        if !state.speech_knowledge_current(&task.id).await {
            state
                .inner
                .lock()
                .await
                .queue
                .fail(&task.id, task.generation, "观众资料已更新或过期");
            continue;
        }
        let connection = {
            let mut inner = state.inner.lock().await;
            if !inner.queue.mark_ready(&task.id, task.generation) {
                continue;
            }
            let Some(bridge) = &inner.bridge else {
                continue;
            };
            let Some(sender) = &bridge.audio else {
                continue;
            };
            let data = (bridge.id.clone(), bridge.control.clone(), sender.clone());
            if !inner.queue.mark_dispatched(&task.id, task.generation) {
                continue;
            }
            data
        };
        let (bridge_id, control, sender) = connection;
        let speak = ServerCommand::Speak {
            utterance_id: task.id.clone(),
            generation: task.generation,
            format,
        };
        // Marking dispatched before the send makes interrupted delivery explicitly unknown.
        if control.try_send(speak).is_err() {
            state.disconnect(&bridge_id).await;
            continue;
        }
        drop(knowledge_gate);
        let chunks = audio.samples.chunks(8192);
        let count = chunks.len();
        for (sequence, samples) in chunks.enumerate() {
            let chunk = AudioChunk {
                utterance_id: task.id.clone(),
                generation: task.generation,
                sequence: sequence as u32,
                samples: samples.to_vec(),
                end: sequence + 1 == count,
            };
            let delivered = tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                result = tokio::time::timeout(Duration::from_secs(5), sender.send(chunk)) => matches!(result, Ok(Ok(()))),
            };
            if !delivered {
                state.disconnect(&bridge_id).await;
                break;
            }
        }
        // Data delivery is not playback completion. Wait for a terminal device receipt.
        let playback_seconds =
            audio.samples.len() as f64 / f64::from(audio.sample_rate) / f64::from(audio.channels);
        let deadline = tokio::time::sleep(Duration::from_secs_f64(playback_seconds + 15.0));
        tokio::pin!(deadline);
        loop {
            let notified = state.wake.notified();
            if terminal(&state, &task).await {
                break;
            }
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                _ = &mut deadline => { state.disconnect(&bridge_id).await; break; }
                _ = notified => {}
            }
        }
    }
}

async fn terminal(state: &AppState, task: &SpeechTask) -> bool {
    state
        .inner
        .lock()
        .await
        .queue
        .get(&task.id)
        .is_none_or(|current| current.status.is_terminal())
}
