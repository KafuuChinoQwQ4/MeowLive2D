//! 模型输出全部校验通过后才修改话题或准备语音；工作代次隔离迟到结果。
use super::{ActiveSpeech, AgentSession, EventStatus, PreparedSpeech, settings::validate_text};
use crate::ports::llm::{AgentDecision, ConversationTurn};
use meowlive_domain::{event::EventKind, speech::SpeechText};
use std::collections::HashSet;

impl AgentSession {
    pub fn resolve(
        &mut self,
        work_id: u64,
        mut decision: AgentDecision,
        speech_id: String,
        now_ms: u64,
    ) -> Result<Option<PreparedSpeech>, String> {
        let flight = self
            .flight
            .as_ref()
            .filter(|flight| flight.id == work_id)
            .ok_or_else(|| "stale agent decision".to_owned())?;
        let expired: Vec<_> = self
            .scheduler
            .records
            .iter()
            .filter(|r| {
                flight.batch.events.iter().any(|e| e.id == r.event.id) && now_ms >= r.expires_at_ms
            })
            .map(|r| r.event.id.clone())
            .collect();
        // A decision is atomic; return still-valid candidates to the queue when part expires.
        if !expired.is_empty() {
            let flight = self.flight.take().expect("checked flight");
            let remaining = flight
                .batch
                .events
                .iter()
                .filter(|e| !expired.contains(&e.id))
                .map(|e| e.id.clone())
                .collect::<Vec<_>>();
            self.scheduler
                .update(&expired, EventStatus::Expired, None, None);
            self.scheduler
                .update(&remaining, EventStatus::Pending, None, None);
            return Ok(None);
        }
        if flight
            .batch
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::RoomEnter))
        {
            let pending = self
                .scheduler
                .records
                .iter()
                .filter(|r| matches!(r.status, EventStatus::Pending | EventStatus::Deciding))
                .count();
            let busy = self
                .interaction
                .busy(&self.settings.interaction, pending, now_ms);
            let important_waiting = self.scheduler.records.iter().any(|r| {
                r.status == EventStatus::Pending && !matches!(r.event.kind, EventKind::RoomEnter)
            });
            if busy || important_waiting {
                let ids = flight
                    .batch
                    .events
                    .iter()
                    .map(|e| e.id.clone())
                    .collect::<Vec<_>>();
                self.flight = None;
                self.scheduler.update(
                    &ids,
                    EventStatus::Skipped,
                    None,
                    Some("生成期间直播间变忙，跳过欢迎"),
                );
                return Ok(None);
            }
        }
        if flight.required_read && decision.text.is_none() && decision.reply_to.is_empty() {
            decision.reply_to = flight.batch.events.iter().map(|e| e.id.clone()).collect();
            decision.text = Some(match flight.batch.events[0].kind {
                EventKind::RoomEnter => "很高兴见到你。".into(),
                _ => "这条留言我收到了。".into(),
            });
        }
        let ids: HashSet<_> = flight
            .batch
            .events
            .iter()
            .map(|event| event.id.as_str())
            .collect();
        let chosen: HashSet<_> = decision.reply_to.iter().map(String::as_str).collect();
        if chosen.len() != decision.reply_to.len() || chosen.iter().any(|id| !ids.contains(id)) {
            return Err("decision reply_to must contain distinct candidate event IDs".into());
        }
        for group in &flight.batch.groups {
            let count = group
                .iter()
                .filter(|id| chosen.contains(id.as_str()))
                .count();
            if count != 0 && count != group.len() {
                return Err("decision must select all or none of a merged gift group".into());
            }
        }
        if let Some(topic) = &decision.topic {
            validate_text("topic", topic, 200, true)?;
        }
        let text = match &decision.text {
            Some(text) => {
                if [
                    "好感度",
                    "好感分",
                    "熟悉度",
                    "内部评分",
                    "affinity score",
                    "familiarity score",
                ]
                .iter()
                .any(|term| text.to_lowercase().contains(term))
                {
                    return Err(
                        "public response must not expose internal relationship scores".into(),
                    );
                }
                if !ids.is_empty() && chosen.is_empty() {
                    return Err("event response must select at least one candidate".into());
                }
                if speech_id.trim().is_empty() {
                    return Err("speech ID must not be empty".into());
                }
                let reply = SpeechText::new(text)
                    .map_err(|error| error.to_string())?
                    .as_str()
                    .to_owned();
                let mut composed = flight
                    .batch
                    .events
                    .iter()
                    .filter(|e| chosen.contains(e.id.as_str()))
                    .map(super::interaction::read_prefix)
                    .collect::<String>();
                composed.push_str(&reply);
                Some(
                    SpeechText::broadcast(composed)
                        .map_err(|e| e.to_string())?
                        .as_str()
                        .to_owned(),
                )
            }
            None => {
                if !chosen.is_empty() {
                    return Err("silent decision must not select events".into());
                }
                None
            }
        };
        let flight = self
            .flight
            .take()
            .expect("validated flight remains present");
        let skipped: Vec<_> = flight
            .batch
            .events
            .iter()
            .filter(|event| !chosen.contains(event.id.as_str()))
            .map(|event| event.id.clone())
            .collect();
        self.scheduler.skipped(&skipped, &flight.batch.targets);
        if let Some(topic) = decision.topic {
            self.settings.topic = topic;
        }
        self.last_error = None;
        self.cooldown(now_ms);
        let Some(text) = text else {
            return Ok(None);
        };
        let user = flight
            .batch
            .events
            .iter()
            .filter(|event| chosen.contains(event.id.as_str()))
            .map(|event| match &event.kind {
                EventKind::Chat { text } => format!("{}：{}", event.viewer, text),
                EventKind::Gift { name, count } => {
                    format!("{} 赠送 {} × {}", event.viewer, name, count)
                }
                EventKind::SuperChat {
                    text, amount_cny, ..
                } => format!("{} 发送 {} 元SC：{}", event.viewer, amount_cny, text),
                EventKind::RoomEnter => format!("{} 进入直播间", event.viewer),
            })
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .take(4000)
            .collect();
        self.scheduler.update(
            &decision.reply_to,
            EventStatus::Queued,
            Some(&speech_id),
            None,
        );
        for event in &flight.batch.events {
            if chosen.contains(event.id.as_str()) && matches!(event.kind, EventKind::RoomEnter) {
                self.interaction.welcomed(event, now_ms);
            }
        }
        self.current = Some(ActiveSpeech {
            id: speech_id,
            events: flight
                .batch
                .events
                .into_iter()
                .filter(|e| chosen.contains(e.id.as_str()))
                .collect(),
            event_ids: decision.reply_to,
            turn: ConversationTurn {
                user,
                assistant: text.clone(),
            },
        });
        Ok(Some(PreparedSpeech { text }))
    }

    pub fn fail(&mut self, work_id: u64, message: String, now_ms: u64) {
        if self
            .flight
            .as_ref()
            .is_none_or(|flight| flight.id != work_id)
        {
            return;
        }
        let flight = self.flight.take().expect("matched flight remains present");
        let ids = flight
            .batch
            .events
            .iter()
            .map(|event| event.id.clone())
            .collect::<Vec<_>>();
        let message: String = message.chars().take(1000).collect();
        self.scheduler
            .update(&ids, EventStatus::Failed, None, Some(&message));
        self.last_error = Some(message);
        self.cooldown(now_ms);
    }
}
