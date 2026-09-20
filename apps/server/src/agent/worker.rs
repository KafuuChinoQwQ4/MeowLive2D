use crate::state::AppState;
use meowlive_application::ports::llm::{AgentDecision, DecisionRequest, LanguageModel, LlmError};
use std::time::Duration;

pub async fn run_agent(state: AppState) {
    loop {
        state.expire_knowledge().await;
        let notified = state.agent_wake.notified();
        let work = {
            let mut inner = state.inner.lock().await;
            state.sync_agent(&mut inner);
            if state.companionship_store.is_none() {
                inner.agent.take_completed();
            }
            if !state
                .resource_changing
                .load(std::sync::atomic::Ordering::Acquire)
                && !state.gpu_busy.load(std::sync::atomic::Ordering::Acquire)
                && !state
                    .model_synthesizer
                    .as_ref()
                    .is_some_and(|synthesizer| synthesizer.is_busy())
                && state
                    .receipts_pending
                    .load(std::sync::atomic::Ordering::Acquire)
                    < 4095
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
        let Some((mut work, cancel, generation)) = work else {
            tokio::select! {_=notified=>{},_=tokio::time::sleep(Duration::from_millis(100))=>{}}
            continue;
        };
        let context_epoch = state
            .knowledge_epoch
            .load(std::sync::atomic::Ordering::Acquire);
        let context_stamp = state.enrich_memories(&mut work.request).await;
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
        let _knowledge = state.knowledge_gate.read().await;
        let stale_revision = if let Some(stamp) = context_stamp {
            !state.knowledge_current(stamp).await
        } else {
            false
        };
        let mut inner = state.inner.lock().await;
        if stale_revision
            || context_epoch
                != state
                    .knowledge_epoch
                    .load(std::sync::atomic::Ordering::Acquire)
        {
            inner.agent.fail(
                work.id,
                "观众资料已更新，本次生成作废".into(),
                state.now_ms(),
            );
            continue;
        }
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
                        let selected = inner.agent.prepared_events(&speech_id);
                        drop(inner);
                        let registration = if let Some(store) = &state.companionship_store {
                            store
                                .register_reply(
                                    &state.config.viewers.scope_id,
                                    &speech_id,
                                    &selected,
                                    crate::viewers::utc_ms(),
                                )
                                .await
                        } else {
                            Ok(())
                        };
                        let mut inner = state.inner.lock().await;
                        if cancel.is_cancelled() || generation != inner.queue.generation() {
                            continue;
                        }
                        if registration.is_err() {
                            inner.agent.speech_failed(
                                &speech_id,
                                "回应关联保存失败，未安排播报".into(),
                                state.now_ms(),
                            );
                            continue;
                        }
                        if let Err(error) = inner.queue.enqueue(&speech_id, prepared.text, voice_id)
                        {
                            inner
                                .agent
                                .speech_failed(&speech_id, error.to_string(), now);
                        } else {
                            if let Some(stamp) = context_stamp {
                                state.knowledge_deadline.fetch_min(
                                    stamp.expires_at_ms,
                                    std::sync::atomic::Ordering::AcqRel,
                                );
                                let alive = inner
                                    .queue
                                    .tasks()
                                    .filter(|t| !t.status.is_terminal())
                                    .map(|t| t.id.clone())
                                    .collect::<std::collections::HashSet<_>>();
                                inner.knowledge.retain(|id, _| alive.contains(id));
                                inner.knowledge.insert(speech_id, stamp);
                            }
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
    async fn managed_tts_is_not_failed_by_the_outer_worker_timeout() {
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
                        viewer_identity: None,
                        occurred_at_ms: 0,
                        gift_metadata: None,
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
        tokio::time::sleep(Duration::from_millis(1_100)).await;
        assert_eq!(
            state.snapshot().await.speeches[0].status,
            meowlive_protocol::control::SpeechStatus::Synthesizing
        );
        assert!(
            synth.is_busy(),
            "managed TTS must still hold ownership while synthesis is running"
        );
        assert!(state.acquire_model_selection().await.is_err());
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "Agent must not start LLM while managed TTS is still running"
        );
        release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if state.snapshot().await.speeches[0].status
                    == meowlive_protocol::control::SpeechStatus::Ready
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("managed synthesis must be delivered after it finishes");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        worker.abort();
        agent.abort();
        server.abort();
    }
}
