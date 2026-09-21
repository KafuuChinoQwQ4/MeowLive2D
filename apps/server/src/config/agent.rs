//! Agent 人设与有界调度配置；运行时始终从暂停状态开始。
use meowlive_application::agent::{AgentLimits, AgentSettings};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentConfig {
    pub persona: String,
    pub topic: String,
    pub proactive_enabled: bool,
    pub cooldown_ms: u64,
    pub interaction: meowlive_protocol::agent::InteractionSettings,
    pub pending_capacity: usize,
    pub history_limit: usize,
    pub dedup_capacity: usize,
    pub event_ttl_ms: u64,
    pub gift_merge_ms: u64,
    pub batch_size: usize,
    pub conversation_limit: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        let settings = AgentSettings::default();
        let limits = AgentLimits::default();
        Self {
            persona: settings.persona,
            topic: settings.topic,
            proactive_enabled: settings.proactive_enabled,
            cooldown_ms: settings.cooldown_ms,
            interaction: meowlive_protocol::agent::InteractionSettings::default(),
            pending_capacity: limits.pending_capacity,
            history_limit: limits.history_limit,
            dedup_capacity: limits.dedup_capacity,
            event_ttl_ms: limits.event_ttl_ms,
            gift_merge_ms: limits.gift_merge_ms,
            batch_size: limits.batch_size,
            conversation_limit: limits.conversation_limit,
        }
    }
}

impl AgentConfig {
    pub fn settings(&self) -> AgentSettings {
        AgentSettings {
            persona: self.persona.clone(),
            topic: self.topic.clone(),
            proactive_enabled: self.proactive_enabled,
            cooldown_ms: self.cooldown_ms,
            interaction: crate::agent::mapping::interaction(self.interaction.clone()),
        }
    }
    pub fn limits(&self) -> AgentLimits {
        AgentLimits {
            pending_capacity: self.pending_capacity,
            history_limit: self.history_limit,
            dedup_capacity: self.dedup_capacity,
            event_ttl_ms: self.event_ttl_ms,
            gift_merge_ms: self.gift_merge_ms,
            batch_size: self.batch_size,
            conversation_limit: self.conversation_limit,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        self.settings().validate()?;
        self.limits().validate()
    }
}
