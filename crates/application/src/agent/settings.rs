//! 人设与调度资源上限的可验证配置。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSettings {
    pub persona: String,
    pub topic: String,
    pub proactive_enabled: bool,
    pub cooldown_ms: u64,
    pub interaction: super::InteractionSettings,
}
impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            persona: "你是一位友好、自然的 Live2D 主播，用简短中文与观众互动。".into(),
            topic: String::new(),
            proactive_enabled: false,
            cooldown_ms: 30_000,
            interaction: super::InteractionSettings::default(),
        }
    }
}
impl AgentSettings {
    pub fn validate(&self) -> Result<(), String> {
        self.interaction.validate()?;
        validate_text("persona", &self.persona, 2000, false)?;
        validate_text("topic", &self.topic, 200, true)?;
        if !(1000..=3_600_000).contains(&self.cooldown_ms) {
            return Err("cooldown must be between 1000 and 3600000 milliseconds".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct AgentLimits {
    pub pending_capacity: usize,
    pub history_limit: usize,
    pub dedup_capacity: usize,
    pub event_ttl_ms: u64,
    pub gift_merge_ms: u64,
    pub batch_size: usize,
    pub conversation_limit: usize,
}
impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            pending_capacity: 128,
            history_limit: 200,
            dedup_capacity: 512,
            event_ttl_ms: 60_000,
            gift_merge_ms: 3000,
            batch_size: 8,
            conversation_limit: 6,
        }
    }
}
impl AgentLimits {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, maximum) in [
            ("pending capacity", self.pending_capacity, 512),
            ("history limit", self.history_limit, 2000),
            ("dedup capacity", self.dedup_capacity, 4096),
            ("batch size", self.batch_size, 16),
            ("conversation limit", self.conversation_limit, 20),
        ] {
            if value == 0 || value > maximum {
                return Err(format!("{name} must be between 1 and {maximum}"));
            }
        }
        if !(1000..=600_000).contains(&self.event_ttl_ms) {
            return Err("event TTL must be between 1000 and 600000 milliseconds".into());
        }
        if self.gift_merge_ms > 10_000 {
            return Err("gift merge window must not exceed 10000 milliseconds".into());
        }
        Ok(())
    }
}

pub(super) fn validate_text(
    label: &str,
    value: &str,
    maximum: usize,
    empty: bool,
) -> Result<(), String> {
    if (!empty && value.trim().is_empty()) || value.chars().count() > maximum {
        return Err(format!(
            "{label} must contain {} to {maximum} characters",
            usize::from(!empty)
        ));
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\t'))
    {
        return Err(format!("{label} contains a control character"));
    }
    Ok(())
}
