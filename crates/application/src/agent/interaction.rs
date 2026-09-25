//! 流量窗口、点名欢迎冷却与确定性原文朗读；不把消息频率当作在线人数。
use meowlive_domain::event::{EventKind, LiveEvent};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChatReadMode {
    #[default]
    Auto,
    All,
    Selective,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionSettings {
    pub chat_read_mode: ChatReadMode,
    pub welcome_enabled: bool,
    pub busy_chat_count: u32,
    pub busy_enter_count: u32,
    pub busy_pending_count: u32,
    pub welcome_cooldown_ms: u32,
    pub welcome_viewer_cooldown_ms: u32,
}

impl Default for InteractionSettings {
    fn default() -> Self {
        Self {
            chat_read_mode: ChatReadMode::Auto,
            welcome_enabled: true,
            busy_chat_count: 6,
            busy_enter_count: 3,
            busy_pending_count: 4,
            welcome_cooldown_ms: 30_000,
            welcome_viewer_cooldown_ms: 600_000,
        }
    }
}

impl InteractionSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=1000).contains(&self.busy_chat_count)
            || !(1..=1000).contains(&self.busy_enter_count)
            || !(1..=512).contains(&self.busy_pending_count)
            || !(1000..=3_600_000).contains(&self.welcome_cooldown_ms)
            || !(1000..=86_400_000).contains(&self.welcome_viewer_cooldown_ms)
        {
            return Err("invalid interaction thresholds or welcome cooldown".into());
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
pub(super) struct InteractionState {
    chats: VecDeque<u64>,
    entries: VecDeque<u64>,
    last_welcome: Option<u64>,
    viewers: HashMap<String, u64>,
}

impl InteractionState {
    pub fn observe(&mut self, event: &LiveEvent, now: u64) {
        self.prune(now);
        let samples = match event.kind {
            EventKind::Chat { .. } => &mut self.chats,
            EventKind::RoomEnter => &mut self.entries,
            _ => return,
        };
        samples.push_back(now);
        // Maximum configurable threshold is 1000; a bounded suffix retains saturation.
        while samples.len() > 1000 {
            samples.pop_front();
        }
    }

    fn prune(&mut self, now: u64) {
        for samples in [&mut self.chats, &mut self.entries] {
            while samples
                .front()
                .is_some_and(|t| now.saturating_sub(*t) >= 60_000)
            {
                samples.pop_front();
            }
        }
        self.viewers
            .retain(|_, at| now.saturating_sub(*at) < 86_400_000);
    }

    pub fn busy(&mut self, settings: &InteractionSettings, pending: usize, now: u64) -> bool {
        self.prune(now);
        self.chats.len() >= settings.busy_chat_count as usize
            || self.entries.len() >= settings.busy_enter_count as usize
            || pending >= settings.busy_pending_count as usize
    }

    pub fn may_welcome(&self, event: &LiveEvent, settings: &InteractionSettings, now: u64) -> bool {
        settings.welcome_enabled
            && self
                .last_welcome
                .is_none_or(|at| now.saturating_sub(at) >= u64::from(settings.welcome_cooldown_ms))
            && self.viewers.get(&viewer_key(event)).is_none_or(|at| {
                now.saturating_sub(*at) >= u64::from(settings.welcome_viewer_cooldown_ms)
            })
    }

    pub fn welcomed(&mut self, event: &LiveEvent, now: u64) {
        self.last_welcome = Some(now);
        if self.viewers.len() >= 4096 {
            if let Some(key) = self
                .viewers
                .iter()
                .min_by_key(|(_, at)| **at)
                .map(|(key, _)| key.clone())
            {
                self.viewers.remove(&key);
            }
        }
        self.viewers.insert(viewer_key(event), now);
    }
}

fn viewer_key(event: &LiveEvent) -> String {
    match &event.viewer_identity {
        Some(identity) => format!("{}:{:?}:{}", event.source, identity, identity.external_id),
        None => format!("{}:anonymous:{}", event.source, event.viewer),
    }
}

pub(crate) fn read_prefix(event: &LiveEvent) -> String {
    match &event.kind {
        EventKind::Chat { text } => spoken_text(text),
        EventKind::SuperChat {
            text, amount_cny, ..
        } => format!(
            "感谢{}的{}元SC。{}",
            event.viewer,
            amount_cny,
            spoken_text(text)
        ),
        EventKind::RoomEnter => format!("欢迎{}，感谢你来到直播间！", event.viewer),
        EventKind::Gift { .. } => String::new(),
    }
}

fn spoken_text(text: &str) -> String {
    let mut spoken = text.to_owned();
    if !spoken
        .chars()
        .last()
        .is_some_and(|character| matches!(character, '。' | '！' | '？' | '!' | '?' | '.' | '…'))
    {
        spoken.push('。');
    }
    spoken
}
