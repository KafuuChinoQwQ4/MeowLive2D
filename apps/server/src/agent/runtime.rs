//! 有界工具循环、实时阶段和取消所有权；完整决策仍交由既有 Agent 状态机验收。
use super::tools::{ToolSet, environment};
use crate::{
    agent_observability::{TraceStepInput, TurnInput},
    llm_runtime::{RuntimeStore, measured_turn},
    state::AppState,
};
use meowlive_application::ports::{
    llm::{AgentDecision, DecisionRequest, LanguageModel, LlmError},
    llm_runtime::{ModelEvent, ModelObserver, ModelOptions, ModelTurn, ToolExchange, ToolResult},
};
use meowlive_protocol::agent_observability::{AgentTraceStepKind, AgentTraceStepStatus};
use meowlive_protocol::llm_runtime::{
    AgentActivitySnapshot, AgentRuntimeSettings, AgentToolActivity,
};
use std::{collections::HashSet, sync::Arc, time::Duration};

pub(super) async fn decide(
    state: &AppState,
    model: &dyn LanguageModel,
    request: DecisionRequest,
    max_retries: u32,
    trace_id: &str,
) -> Result<AgentDecision, LlmError> {
    let mut activity = ActivityGuard::new_for(state, trace_id.to_owned());
    let (settings, search_key) = state.llm_runtime.settings_with_search_key();
    let tools = ToolSet::new(&settings, search_key);
    let result = tokio::time::timeout(
        Duration::from_secs(state.config.llm.timeout_seconds),
        run_observed(
            state,
            model,
            request,
            max_retries,
            &settings,
            tools,
            &activity,
            Some(trace_id),
        ),
    )
    .await
    .unwrap_or_else(|_| Err(LlmError::new("LLM 决策超时（包含工具和重试）", false)));
    activity.finish(
        if result.is_ok() {
            "completed"
        } else {
            "failed"
        },
        if result.is_ok() {
            "回应已生成"
        } else {
            "本次生成失败或超时"
        },
    );
    result
}

#[cfg(test)]
async fn run(
    state: &AppState,
    model: &dyn LanguageModel,
    request: DecisionRequest,
    max_retries: u32,
    settings: &AgentRuntimeSettings,
    tools: ToolSet,
    activity: &ActivityGuard,
) -> Result<AgentDecision, LlmError> {
    run_observed(
        state,
        model,
        request,
        max_retries,
        settings,
        tools,
        activity,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_observed(
    state: &AppState,
    model: &dyn LanguageModel,
    request: DecisionRequest,
    max_retries: u32,
    settings: &AgentRuntimeSettings,
    tools: ToolSet,
    activity: &ActivityGuard,
    trace_id: Option<&str>,
) -> Result<AgentDecision, LlmError> {
    let mut options = ModelOptions {
        tools: tools.definitions.clone(),
        cache_enabled: settings.cache_enabled,
        stream: settings.streaming,
        reasoning_effort: state
            .config
            .llm
            .reasoning_effort
            .parse()
            .map_err(|_| LlmError::new("推理强度配置无效", false))?,
        reasoning_provider: state.config.llm.provider.clone(),
        environment: if settings.environment_enabled {
            Some(environment(state).await.to_string())
        } else {
            None
        },
        observer: Some(Arc::new(ActivityObserver {
            store: state.llm_runtime.clone(),
            id: activity.id.clone(),
        })),
        ..ModelOptions::default()
    };
    let max_rounds = settings.max_tool_rounds.clamp(1, 3);
    for round in 0..=max_rounds {
        activity.update(|value| {
            value.phase = "thinking".into();
            value.message = if round == 0 {
                "正在生成回应"
            } else {
                "正在结合工具结果生成回应"
            }
            .into();
            value.output_characters = 0;
        });
        // After the last permitted tool round, request one final answer without tools.
        if round == max_rounds {
            options.tool_choice_none = true;
        }
        let (turn, observed_turn_id) = retry_turn(
            state,
            model,
            &request,
            &options,
            max_retries,
            round,
            trace_id,
        )
        .await?;
        if let Some(decision) = turn.decision {
            if !turn.tool_calls.is_empty() {
                return Err(LlmError::new("模型同时返回最终决策与工具调用", false));
            }
            if let Some(trace_id) = trace_id {
                state.agent_observability.append_step(
                    trace_id,
                    TraceStepInput {
                        kind: AgentTraceStepKind::DecisionReceived,
                        status: AgentTraceStepStatus::Completed,
                        message: "模型已返回最终决策".into(),
                        turn_id: observed_turn_id,
                        tool_name: None,
                        speech_id: None,
                        elapsed_ms: None,
                        sources: vec![],
                    },
                );
            }
            return Ok(decision);
        }
        if round == max_rounds {
            return Err(LlmError::new("已达到工具调用轮次上限", false));
        }
        if turn.tool_calls.is_empty() || turn.tool_calls.len() > 4 {
            return Err(LlmError::new("模型工具调用数量无效", false));
        }
        let continuation = turn
            .continuation
            .filter(|value| !value.is_empty() && value.len() <= 262_144)
            .ok_or_else(|| LlmError::new("工具调用缺少有效上下文", false))?;
        let mut ids = HashSet::new();
        if turn.tool_calls.iter().any(|call| {
            call.id.is_empty()
                || call.id.len() > 256
                || call.name.is_empty()
                || call.name.len() > 64
                || call.name.chars().any(char::is_control)
                || !ids.insert(call.id.clone())
        }) {
            return Err(LlmError::new("工具调用标识无效或重复", false));
        }
        let mut results = Vec::new();
        for call in turn.tool_calls {
            let started = tokio::time::Instant::now();
            let observed_name = tools.observed_name(&call.name);
            let index = state.llm_runtime.activity().tools.len();
            activity.update(|value| {
                value.phase = "tool".into();
                value.tool_round = round + 1;
                value.message = "正在查询资料或环境状态".into();
                value.tools.push(AgentToolActivity {
                    name: observed_name.clone(),
                    status: "running".into(),
                    elapsed_ms: 0,
                    sources: vec![],
                });
            });
            let mut trace_tool = ToolTraceGuard::start(
                state,
                trace_id,
                observed_turn_id.clone(),
                observed_name.clone(),
            );
            let (result,sources)=tokio::time::timeout(Duration::from_secs(u64::from(settings.tool_timeout_seconds.clamp(1,8))),tools.execute(state,&call)).await
                .unwrap_or_else(|_|(ToolResult{call_id:call.id.clone(),name:call.name.clone(),content:r#"{"available":false,"error":"工具查询超时，请使用已有资料或说明暂时无法确认"}"#.into(),is_error:true},vec![]));
            activity.update(|value| {
                if let Some(tool) = value.tools.get_mut(index) {
                    tool.status = if result.is_error {
                        "failed"
                    } else {
                        "completed"
                    }
                    .into();
                    tool.elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
                    tool.sources = sources.clone();
                }
            });
            trace_tool.finish(result.is_error, sources.clone());
            results.push(result);
        }
        options.exchanges.push(ToolExchange {
            continuation,
            results,
        });
    }
    unreachable!("final turn must return")
}

struct ToolTraceGuard<'a> {
    state: &'a AppState,
    trace_id: Option<String>,
    turn_id: Option<String>,
    tool_name: String,
    started: tokio::time::Instant,
    finished: bool,
}

impl<'a> ToolTraceGuard<'a> {
    fn start(
        state: &'a AppState,
        trace_id: Option<&str>,
        turn_id: Option<String>,
        tool_name: String,
    ) -> Self {
        if let Some(trace_id) = trace_id {
            state.agent_observability.append_step(
                trace_id,
                TraceStepInput {
                    kind: AgentTraceStepKind::ToolStarted,
                    status: AgentTraceStepStatus::Running,
                    message: if tool_name == "unknown_tool" {
                        "开始处理未识别的工具调用".into()
                    } else {
                        format!("开始调用工具 {tool_name}")
                    },
                    turn_id: turn_id.clone(),
                    tool_name: Some(tool_name.clone()),
                    speech_id: None,
                    elapsed_ms: None,
                    sources: vec![],
                },
            );
        }
        Self {
            state,
            trace_id: trace_id.map(str::to_owned),
            turn_id,
            tool_name,
            started: tokio::time::Instant::now(),
            finished: false,
        }
    }

    fn finish(&mut self, failed: bool, sources: Vec<String>) {
        let Some(trace_id) = self.trace_id.as_deref() else {
            self.finished = true;
            return;
        };
        self.state.agent_observability.append_step(
            trace_id,
            TraceStepInput {
                kind: AgentTraceStepKind::ToolFinished,
                status: if failed {
                    AgentTraceStepStatus::Failed
                } else {
                    AgentTraceStepStatus::Completed
                },
                message: if failed {
                    format!("工具 {} 未成功", self.tool_name)
                } else {
                    format!("工具 {} 调用完成", self.tool_name)
                },
                turn_id: self.turn_id.clone(),
                tool_name: Some(self.tool_name.clone()),
                speech_id: None,
                elapsed_ms: Some(self.started.elapsed().as_millis().min(u64::MAX as u128) as u64),
                sources,
            },
        );
        self.finished = true;
    }
}

impl Drop for ToolTraceGuard<'_> {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Some(trace_id) = self.trace_id.as_deref() {
            self.state.agent_observability.append_step(
                trace_id,
                TraceStepInput {
                    kind: AgentTraceStepKind::ToolFinished,
                    status: AgentTraceStepStatus::Cancelled,
                    message: format!("工具 {} 调用被取消", self.tool_name),
                    turn_id: self.turn_id.clone(),
                    tool_name: Some(self.tool_name.clone()),
                    speech_id: None,
                    elapsed_ms: Some(
                        self.started.elapsed().as_millis().min(u64::MAX as u128) as u64
                    ),
                    sources: vec![],
                },
            );
        }
    }
}

async fn retry_turn(
    state: &AppState,
    model: &dyn LanguageModel,
    request: &DecisionRequest,
    options: &ModelOptions,
    max_retries: u32,
    tool_round: u32,
    trace_id: Option<&str>,
) -> Result<(ModelTurn, Option<String>), LlmError> {
    for attempt in 0..=max_retries {
        let turn_id = trace_id.and_then(|trace_id| {
            state.agent_observability.start_turn(
                trace_id,
                TurnInput {
                    tool_round,
                    retry_attempt: attempt,
                    provider: state.config.llm.provider.clone(),
                    api_format: state.config.llm.api_format.clone(),
                    model: state.config.llm.model.clone(),
                },
            )
        });
        let result = if let (Some(trace_id), Some(turn_id)) = (trace_id, turn_id.as_ref()) {
            crate::llm_runtime::measured_observed_turn(
                state,
                model,
                request.clone(),
                options.clone(),
                Some(crate::llm_runtime::TurnObservation {
                    trace_id: trace_id.into(),
                    turn_id: turn_id.clone(),
                    tool_round,
                    retry_attempt: attempt,
                }),
            )
            .await
        } else {
            measured_turn(state, model, request.clone(), options.clone()).await
        };
        match result {
            Err(error) if error.retryable && attempt < max_retries => {
                tokio::time::sleep(Duration::from_millis(100)).await
            }
            result => return result.map(|turn| (turn, turn_id)),
        }
    }
    unreachable!("each final attempt returns")
}

struct ActivityGuard {
    store: Arc<RuntimeStore>,
    id: String,
    finished: bool,
}
impl ActivityGuard {
    #[cfg(test)]
    fn new(state: &AppState) -> Self {
        Self::new_for(state, uuid::Uuid::new_v4().to_string())
    }
    fn new_for(state: &AppState, id: String) -> Self {
        let now = crate::viewers::utc_ms();
        state.llm_runtime.set_activity(AgentActivitySnapshot {
            run_id: Some(id.clone()),
            phase: "thinking".into(),
            started_at_ms: Some(now),
            updated_at_ms: now,
            output_characters: 0,
            tool_round: 0,
            tools: vec![],
            message: "正在生成回应".into(),
        });
        Self {
            store: state.llm_runtime.clone(),
            id,
            finished: false,
        }
    }
    fn update(&self, update: impl FnOnce(&mut AgentActivitySnapshot)) {
        update_activity(&self.store, &self.id, update);
    }
    fn finish(&mut self, phase: &str, message: &str) {
        self.update(|value| {
            value.phase = phase.into();
            value.message = message.into();
            for tool in &mut value.tools {
                if tool.status == "running" {
                    tool.status = "failed".into();
                }
            }
        });
        self.finished = true;
    }
}
impl Drop for ActivityGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.update(|value| {
                value.phase = "cancelled".into();
                value.message = "本次生成已取消".into();
                for tool in &mut value.tools {
                    if tool.status == "running" {
                        tool.status = "failed".into();
                    }
                }
            });
        }
    }
}
fn update_activity(
    store: &RuntimeStore,
    id: &str,
    update: impl FnOnce(&mut AgentActivitySnapshot),
) {
    store.update_activity_for(id, |value| {
        update(value);
        value.updated_at_ms = crate::viewers::utc_ms();
    });
}
struct ActivityObserver {
    store: Arc<RuntimeStore>,
    id: String,
}
impl ModelObserver for ActivityObserver {
    fn on_event(&self, event: ModelEvent) {
        match event {
            ModelEvent::FirstToken => update_activity(&self.store, &self.id, |value| {
                value.phase = "receiving".into();
                value.message = "正在接收模型输出".into();
            }),
            ModelEvent::OutputProgress { characters } => {
                update_activity(&self.store, &self.id, |value| {
                    value.phase = "receiving".into();
                    value.output_characters = characters;
                })
            }
            ModelEvent::Usage(_) => {}
        }
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
