//! 模型输出全部校验通过后才修改话题或准备语音；工作代次隔离迟到结果。
use super::{ActiveSpeech, AgentSession, EventStatus, PreparedSpeech, settings::validate_text};
use crate::ports::llm::{AgentDecision, ConversationTurn};
use meowlive_domain::{event::EventKind, speech::SpeechText};
use std::collections::HashSet;

impl AgentSession {
    pub fn resolve(
        &mut self,
        work_id: u64,
        decision: AgentDecision,
        speech_id: String,
        now_ms: u64,
    ) -> Result<Option<PreparedSpeech>, String> {
        let flight = self
            .flight
            .as_ref()
            .filter(|flight| flight.id == work_id)
            .ok_or_else(|| "stale agent decision".to_owned())?;
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
                if !ids.is_empty() && chosen.is_empty() {
                    return Err("event response must select at least one candidate".into());
                }
                if speech_id.trim().is_empty() {
                    return Err("speech ID must not be empty".into());
                }
                Some(
                    SpeechText::new(text)
                        .map_err(|error| error.to_string())?
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
        self.scheduler
            .update(&skipped, EventStatus::Skipped, None, None);
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
        self.current = Some(ActiveSpeech {
            id: speech_id,
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
