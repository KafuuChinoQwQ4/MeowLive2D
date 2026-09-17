use crate::state::AppState;
use meowlive_application::ports::llm::{AgentDecision, DecisionRequest, LanguageModel, LlmError};
use std::time::Duration;

pub async fn run_agent(state: AppState) {
    loop {
        let notified = state.agent_wake.notified();
        let work = {
            let mut inner = state.inner.lock().await;
            state.sync_agent(&mut inner);
            if !state
                .resource_changing
                .load(std::sync::atomic::Ordering::Acquire)
                && !state.gpu_busy.load(std::sync::atomic::Ordering::Acquire)
                && !state
                    .model_synthesizer
                    .as_ref()
                    .is_some_and(|synthesizer| synthesizer.is_busy())
                && state.model.is_some()
                && inner.queue.is_connected()
                && !inner.queue.tasks().any(|task| !task.status.is_terminal())
            {
                inner
                    .agent
                    .begin(state.now_ms())
                    .map(|work| (work, inner.agent_cancel.clone(), inner.queue.generation()))
            } else {
                None
            }
        };
        let Some((work, cancel, generation)) = work else {
            tokio::select! {_=notified=>{},_=tokio::time::sleep(Duration::from_millis(100))=>{}}
            continue;
        };
        let model = state
            .model
            .as_ref()
            .expect("checked before claiming decision");
        // This deadline includes every retry and its delay; cancellation wins over
        // model completion, and admission below rechecks the same fence under lock.
        let result = tokio::select! {
            biased;
            _=cancel.cancelled()=>continue,
            result=tokio::time::timeout(Duration::from_secs(state.config.llm.timeout_seconds),
                decide(model.as_ref(),work.request,state.config.llm.max_retries))=> {
                result.unwrap_or_else(|_|Err(LlmError::new("LLM 决策超时",false)))
            },
        };
        let resources = state.resources.clone();
        let voice_id =
            match tokio::task::spawn_blocking(move || resources.snapshot().active_voice_id).await {
                Ok(id) => id,
                Err(_) => String::new(),
            };
        let mut inner = state.inner.lock().await;
        if cancel.is_cancelled() || generation != inner.queue.generation() {
            continue;
        }
        let now = state.now_ms();
        match result {
            Ok(decision) => {
                let speech_id = uuid::Uuid::new_v4().to_string();
                match inner
                    .agent
                    .resolve(work.id, decision, speech_id.clone(), now)
                {
                    Ok(Some(prepared)) => {
                        if let Err(error) = inner.queue.enqueue(&speech_id, prepared.text, voice_id)
                        {
                            inner
                                .agent
                                .speech_failed(&speech_id, error.to_string(), now);
                        } else {
                            state.wake.notify_one();
                        }
                    }
                    Ok(None) => {}
                    Err(error) => inner.agent.fail(work.id, error, now),
                }
            }
            Err(error) => inner.agent.fail(work.id, error.message, now),
        }
    }
}

async fn decide(
    model: &dyn LanguageModel,
    request: DecisionRequest,
    max_retries: u32,
) -> Result<AgentDecision, LlmError> {
    for attempt in 0..=max_retries {
        match model.decide(request.clone()).await {
            Err(error) if error.retryable && attempt < max_retries => {
                tokio::time::sleep(Duration::from_millis(100)).await
            }
            result => return result,
        }
    }
    unreachable!("each final attempt returns its result")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, routing::get};
    use meowlive_adapters::speech::model_synthesizer::ModelSynthesizer;
    use meowlive_application::{
        ports::{
            llm::DecisionFuture,
            speech::{PcmAudio, SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
        },
        training::TrainingManager,
    };
    use meowlive_domain::event::{EventKind, LiveEvent};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::sync::Notify;

    struct HeldSpeech {
        entered: Arc<Notify>,
        release: Arc<Notify>,
    }
    impl SpeechSynthesizer for HeldSpeech {
        fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
            Box::pin(async move {
                self.entered.notify_one();
                self.release.notified().await;
                Ok(PcmAudio {
                    sample_rate: 8000,
                    channels: 1,
                    samples: vec![1; 800],
                })
            })
        }
    }
    struct CountingModel(Arc<AtomicUsize>);
    impl LanguageModel for CountingModel {
        fn decide(&self, request: DecisionRequest) -> DecisionFuture<'_> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                Ok(AgentDecision {
                    reply_to: request.events.into_iter().map(|event| event.id).collect(),
                    text: None,
                    topic: None,
                })
            })
        }
    }

    #[tokio::test]
    async fn agent_waits_for_owned_tts_after_speech_worker_timeout() {
        async fn weights() -> Json<serde_json::Value> {
            Json(serde_json::json!({"message":"success"}))
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/set_gpt_weights", get(weights))
                    .route("/set_sovits_weights", get(weights)),
            )
            .await
            .unwrap();
        });
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let synth = Arc::new(
            ModelSynthesizer::new(
                Arc::new(HeldSpeech {
                    entered: entered.clone(),
                    release: release.clone(),
                }),
                Arc::new(TrainingManager::disabled()),
                &endpoint,
                "/default.ckpt".into(),
                "/default.pth".into(),
                Duration::from_secs(2),
            )
            .unwrap(),
        );
        let mut config = crate::config::AppConfig::default();
        config.speech.timeout_seconds = 1;
        let mut state = AppState::with_model(
            config,
            synth.clone(),
            Some(Arc::new(CountingModel(calls.clone()))),
        );
        state.model_synthesizer = Some(synth.clone());
        {
            let mut inner = state.inner.lock().await;
            inner.queue.set_connected(true);
            inner
                .queue
                .enqueue("manual-speech", "测试播报", "default")
                .unwrap();
            inner
                .agent
                .submit(
                    LiveEvent {
                        id: "pending-event".into(),
                        source: "test".into(),
                        viewer: "观众".into(),
                        occurred_at_ms: 0,
                        kind: EventKind::Chat {
                            text: "请回应我".into(),
                        },
                    },
                    0,
                )
                .unwrap();
            inner.agent.set_paused(false, 0);
        }
        let worker = tokio::spawn(crate::worker::run_worker(state.clone()));
        let agent = tokio::spawn(run_agent(state.clone()));
        tokio::time::timeout(Duration::from_secs(2), entered.notified())
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if state.snapshot().await.speeches[0].status
                    == meowlive_protocol::control::SpeechStatus::Failed
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(
            synth.is_busy(),
            "detached TTS must still hold ownership after timeout"
        );
        assert!(state.acquire_model_selection().await.is_err());
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "Agent started LLM while detached TTS was still running"
        );
        release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), async {
            while calls.load(Ordering::SeqCst) == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("Agent must resume after owned TTS finishes");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        worker.abort();
        agent.abort();
        server.abort();
    }
}
