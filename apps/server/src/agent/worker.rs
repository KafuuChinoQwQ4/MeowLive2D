use crate::agent_observability::TraceStepInput;
use crate::state::AppState;
use meowlive_application::agent::{BeginDecision, DecisionResolution};
#[cfg(test)]
use meowlive_application::ports::llm::{AgentDecision, DecisionRequest, LanguageModel};
use meowlive_protocol::agent_observability::{
    AgentSchedulerBlockReason, AgentTraceStatus, AgentTraceStepKind, AgentTraceStepStatus,
};
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
            let admission = super::admission::assess_locked(&state, &mut inner);
            state.agent_observability.set_scheduler(admission.clone());
            // No eligible events can still contain welcomes to retire as skipped.
            if admission.ready
                || admission.block_reason == AgentSchedulerBlockReason::NoEligibleEvents
            {
                match inner.agent.begin_decision(state.now_ms()) {
                    BeginDecision::Work(work) => Some((
                        work,
                        inner.agent_cancel.clone(),
                        inner.queue.generation(),
                        state
                            .knowledge_epoch
                            .load(std::sync::atomic::Ordering::Acquire),
                    )),
                    BeginDecision::Waiting(_) => None,
                }
            } else {
                None
            }
        };
        let Some((mut work, cancel, generation, context_epoch)) = work else {
            tokio::select! {_=notified=>{},_=tokio::time::sleep(Duration::from_millis(100))=>{}}
            continue;
        };
        let trace_id = state.agent_observability.start_trace(
            if work.request.events.is_empty() {
                "proactive"
            } else {
                "live_events"
            },
            work.request
                .events
                .iter()
                .map(super::mapping::trace_event)
                .collect(),
        );
        let context_stamp = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                finish_cancelled(&state, &trace_id, context_epoch);
                continue;
            }
            stamp = state.enrich_memories(&mut work.request) => stamp,
        };
        state.agent_observability.append_step(
            &trace_id,
            TraceStepInput {
                kind: AgentTraceStepKind::ContextReady,
                status: AgentTraceStepStatus::Completed,
                message: format!(
                    "上下文已准备，使用 {} 条观众资料",
                    work.request.memory_context.len()
                ),
                turn_id: None,
                tool_name: None,
                speech_id: None,
                elapsed_ms: None,
                sources: vec![],
            },
        );
        let model = state
            .model
            .as_ref()
            .expect("checked before claiming decision");
        // This deadline includes every retry and its delay; cancellation wins over
        // model completion, and admission below rechecks the same fence under lock.
        let result = tokio::select! {
            biased;
            _=cancel.cancelled()=>{
                finish_cancelled(&state, &trace_id, context_epoch);
                continue
            },
            result=super::runtime::decide(&state,model.as_ref(),work.request,state.config.llm.max_retries,&trace_id)=>result,
        };
        let resources = state.resources.clone();
        let voice_result = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                finish_cancelled(&state, &trace_id, context_epoch);
                continue;
            }
            result = tokio::task::spawn_blocking(move || resources.snapshot().active_voice_id) => result,
        };
        let voice_id = match voice_result {
            Ok(id) => id,
            Err(_) => String::new(),
        };
        let _knowledge = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                finish_cancelled(&state, &trace_id, context_epoch);
                continue;
            }
            gate = state.knowledge_gate.read() => gate,
        };
        let stale_revision = if let Some(stamp) = context_stamp {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    finish_cancelled(&state, &trace_id, context_epoch);
                    continue;
                }
                current = state.knowledge_current(stamp) => !current,
            }
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
            state.agent_observability.finish_trace(
                &trace_id,
                AgentTraceStatus::Failed,
                "观众资料已更新，本次生成作废",
            );
            continue;
        }
        if cancel.is_cancelled() || generation != inner.queue.generation() {
            state.agent_observability.finish_trace(
                &trace_id,
                AgentTraceStatus::Cancelled,
                "任务代次已经变化，本次结果不再使用",
            );
            continue;
        }
        let now = state.now_ms();
        match result {
            Ok(decision) => {
                state.agent_observability.append_step(
                    &trace_id,
                    TraceStepInput {
                        kind: AgentTraceStepKind::ValidationFinished,
                        status: AgentTraceStepStatus::Running,
                        message: "正在校验模型决策".into(),
                        turn_id: None,
                        tool_name: None,
                        speech_id: None,
                        elapsed_ms: None,
                        sources: vec![],
                    },
                );
                let speech_id = uuid::Uuid::new_v4().to_string();
                match inner
                    .agent
                    .resolve_detailed(work.id, decision, speech_id.clone(), now)
                {
                    Ok(DecisionResolution::Speech(prepared)) => {
                        state.agent_observability.append_step(
                            &trace_id,
                            TraceStepInput {
                                kind: AgentTraceStepKind::ValidationFinished,
                                status: AgentTraceStepStatus::Completed,
                                message: "决策校验通过".into(),
                                turn_id: None,
                                tool_name: None,
                                speech_id: None,
                                elapsed_ms: None,
                                sources: vec![],
                            },
                        );
                        let selected = inner.agent.prepared_events(&speech_id);
                        drop(inner);
                        let registration = tokio::select! {
                            biased;
                            _ = cancel.cancelled() => {
                                let mut inner = state.inner.lock().await;
                                inner.agent.speech_cancelled(&speech_id, state.now_ms());
                                finish_cancelled(&state, &trace_id, context_epoch);
                                continue;
                            }
                            registration = async {
                                if let Some(store) = &state.companionship_store {
                                    store.register_reply(
                                        &state.config.viewers.scope_id,
                                        &speech_id,
                                        &selected,
                                        crate::viewers::utc_ms(),
                                    ).await
                                } else {
                                    Ok(())
                                }
                            } => registration,
                        };
                        let mut inner = state.inner.lock().await;
                        if cancel.is_cancelled() || generation != inner.queue.generation() {
                            inner.agent.speech_cancelled(&speech_id, state.now_ms());
                            state.agent_observability.finish_trace(
                                &trace_id,
                                AgentTraceStatus::Cancelled,
                                "任务在保存回应关联时被取消",
                            );
                            continue;
                        }
                        if registration.is_err() {
                            inner.agent.speech_failed(
                                &speech_id,
                                "回应关联保存失败，未安排播报".into(),
                                state.now_ms(),
                            );
                            state.agent_observability.finish_trace(
                                &trace_id,
                                AgentTraceStatus::Failed,
                                "回应关联保存失败，未安排播报",
                            );
                            continue;
                        }
                        if let Err(error) =
                            inner
                                .queue
                                .enqueue_broadcast(&speech_id, prepared.text, voice_id)
                        {
                            inner
                                .agent
                                .speech_failed(&speech_id, error.to_string(), now);
                            state.agent_observability.finish_trace(
                                &trace_id,
                                AgentTraceStatus::Failed,
                                "语音任务加入队列失败",
                            );
                        } else {
                            state.agent_observability.bind_speech(&trace_id, &speech_id);
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
                    Ok(resolution) => {
                        let (status, step_status, message) = match resolution {
                            DecisionResolution::Silent => (
                                AgentTraceStatus::Completed,
                                AgentTraceStepStatus::Completed,
                                "决策已验收，本轮无需播报",
                            ),
                            DecisionResolution::Expired => (
                                AgentTraceStatus::Cancelled,
                                AgentTraceStepStatus::Cancelled,
                                "触发事件已过期，本次生成作废",
                            ),
                            DecisionResolution::PolicySkipped => (
                                AgentTraceStatus::Cancelled,
                                AgentTraceStepStatus::Cancelled,
                                "生成期间直播间变忙，本次欢迎已跳过",
                            ),
                            DecisionResolution::Speech(_) => unreachable!("handled above"),
                        };
                        state.agent_observability.append_step(
                            &trace_id,
                            TraceStepInput {
                                kind: AgentTraceStepKind::ValidationFinished,
                                status: step_status,
                                message: message.into(),
                                turn_id: None,
                                tool_name: None,
                                speech_id: None,
                                elapsed_ms: None,
                                sources: vec![],
                            },
                        );
                        state
                            .agent_observability
                            .finish_trace(&trace_id, status, message);
                    }
                    Err(error) => {
                        inner.agent.fail(work.id, error, now);
                        state.agent_observability.append_step(
                            &trace_id,
                            TraceStepInput {
                                kind: AgentTraceStepKind::ValidationFinished,
                                status: AgentTraceStepStatus::Failed,
                                message: "模型决策未通过业务校验".into(),
                                turn_id: None,
                                tool_name: None,
                                speech_id: None,
                                elapsed_ms: None,
                                sources: vec![],
                            },
                        );
                        state.agent_observability.finish_trace(
                            &trace_id,
                            AgentTraceStatus::Failed,
                            "模型决策未通过业务校验",
                        );
                    }
                }
            }
            Err(error) => {
                inner.agent.fail(work.id, error.message, now);
                state.agent_observability.finish_trace(
                    &trace_id,
                    AgentTraceStatus::Failed,
                    "模型生成失败或超时",
                );
            }
        }
    }
}

fn finish_cancelled(state: &AppState, trace_id: &str, context_epoch: u64) {
    let context_invalidated = context_epoch
        != state
            .knowledge_epoch
            .load(std::sync::atomic::Ordering::Acquire);
    state.agent_observability.finish_trace(
        trace_id,
        if context_invalidated {
            AgentTraceStatus::Failed
        } else {
            AgentTraceStatus::Cancelled
        },
        if context_invalidated {
            "观众资料已更新，本次生成作废"
        } else {
            "Agent 操作取消了本轮任务"
        },
    );
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

#[cfg(test)]
#[path = "observation_tests.rs"]
mod observation_tests;
