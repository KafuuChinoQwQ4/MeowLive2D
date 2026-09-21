//! 有界工具循环、实时阶段和取消所有权；完整决策仍交由既有 Agent 状态机验收。
use super::tools::{ToolSet, environment};
use crate::{
    llm_runtime::{RuntimeStore, measured_turn},
    state::AppState,
};
use meowlive_application::ports::{
    llm::{AgentDecision, DecisionRequest, LanguageModel, LlmError},
    llm_runtime::{ModelEvent, ModelObserver, ModelOptions, ModelTurn, ToolExchange, ToolResult},
};
use meowlive_protocol::llm_runtime::{
    AgentActivitySnapshot, AgentRuntimeSettings, AgentToolActivity,
};
use std::{collections::HashSet, sync::Arc, time::Duration};

pub(super) async fn decide(
    state: &AppState,
    model: &dyn LanguageModel,
    request: DecisionRequest,
    max_retries: u32,
) -> Result<AgentDecision, LlmError> {
    let mut activity = ActivityGuard::new(state);
    let (settings, search_key) = state.llm_runtime.settings_with_search_key();
    let tools = ToolSet::new(&settings, search_key);
    let result = tokio::time::timeout(
        Duration::from_secs(state.config.llm.timeout_seconds),
        run(
            state,
            model,
            request,
            max_retries,
            &settings,
            tools,
            &activity,
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

async fn run(
    state: &AppState,
    model: &dyn LanguageModel,
    request: DecisionRequest,
    max_retries: u32,
    settings: &AgentRuntimeSettings,
    tools: ToolSet,
    activity: &ActivityGuard,
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
        let turn = retry_turn(state, model, &request, &options, max_retries).await?;
        if let Some(decision) = turn.decision {
            if !turn.tool_calls.is_empty() {
                return Err(LlmError::new("模型同时返回最终决策与工具调用", false));
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
            let index = state.llm_runtime.activity().tools.len();
            activity.update(|value| {
                value.phase = "tool".into();
                value.tool_round = round + 1;
                value.message = "正在查询资料或环境状态".into();
                value.tools.push(AgentToolActivity {
                    name: call.name.clone(),
                    status: "running".into(),
                    elapsed_ms: 0,
                    sources: vec![],
                });
            });
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
                    tool.sources = sources;
                }
            });
            results.push(result);
        }
        options.exchanges.push(ToolExchange {
            continuation,
            results,
        });
    }
    unreachable!("final turn must return")
}

async fn retry_turn(
    state: &AppState,
    model: &dyn LanguageModel,
    request: &DecisionRequest,
    options: &ModelOptions,
    max_retries: u32,
) -> Result<ModelTurn, LlmError> {
    for attempt in 0..=max_retries {
        match measured_turn(state, model, request.clone(), options.clone()).await {
            Err(error) if error.retryable && attempt < max_retries => {
                tokio::time::sleep(Duration::from_millis(100)).await
            }
            result => return result,
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
    fn new(state: &AppState) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
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
