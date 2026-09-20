//! 后台提取/嵌入独立预算，删除与修正通过同一版本栅栏取消旧上下文。
use crate::state::AppState;
use meowlive_application::{
    agent::AgentPhase,
    ports::{llm::DecisionRequest, memory_store::QueryEmbedding},
};
use std::{sync::atomic::Ordering, time::Duration};

#[derive(Clone, Copy, Debug)]
pub(crate) struct KnowledgeStamp {
    pub revision: i64,
    pub epoch: u64,
    pub expires_at_ms: i64,
}
impl KnowledgeStamp {
    fn is_current(self, revision: i64, epoch: u64, now: i64) -> bool {
        self.revision == revision && self.epoch == epoch && now < self.expires_at_ms
    }
}
pub async fn run_memory(state: AppState) {
    let mut maintenance_at = 0;
    loop {
        if state.stopping.is_cancelled() {
            break;
        }
        let now = crate::viewers::utc_ms() as i64;
        if let Some(store) = &state.memory_store {
            if now >= maintenance_at {
                let _gate = state.knowledge_gate.write().await;
                let before = store.revision(&state.config.viewers.scope_id).await.ok();
                let _ = store.maintenance(&state.config.viewers.scope_id, now).await;
                if before != store.revision(&state.config.viewers.scope_id).await.ok() {
                    state.invalidate_knowledge().await;
                }
                maintenance_at = now + 60_000;
            }
            let busy = {
                let mut inner = state.inner.lock().await;
                matches!(
                    inner.agent.view(state.now_ms()).phase,
                    AgentPhase::Deciding | AgentPhase::Speaking
                )
            };
            if !busy && !state.gpu_busy.load(Ordering::Acquire) {
                if let Some(extractor) = &state.memory_extractor {
                    if let Ok(Some(job)) =
                        store.claim_job(&state.config.viewers.scope_id, now).await
                    {
                        match extractor.extract(&job.sources).await {
                            Ok(candidates) => {
                                let _gate = state.knowledge_gate.write().await;
                                let before =
                                    store.revision(&state.config.viewers.scope_id).await.ok();
                                if store
                                    .finish_job(
                                        &state.config.viewers.scope_id,
                                        &job,
                                        &candidates,
                                        crate::viewers::utc_ms() as i64,
                                    )
                                    .await
                                    .is_err()
                                {
                                    let _ = store
                                        .fail_job(&state.config.viewers.scope_id, &job, now)
                                        .await;
                                }
                                if before
                                    != store.revision(&state.config.viewers.scope_id).await.ok()
                                {
                                    state.invalidate_knowledge().await;
                                }
                            }
                            Err(_) => {
                                let _ = store
                                    .fail_job(&state.config.viewers.scope_id, &job, now)
                                    .await;
                            }
                        }
                    }
                }
                if let Some(embedder) = &state.memory_embedder {
                    if let Ok(Some(job)) = store
                        .claim_embedding(
                            &state.config.viewers.scope_id,
                            &state.config.memory.embedding_model,
                            state.config.memory.embedding_dimensions,
                            now,
                        )
                        .await
                    {
                        if let Ok(batch) = embedder.embed(std::slice::from_ref(&job.text)).await {
                            let _ = store
                                .store_embedding(
                                    &state.config.viewers.scope_id,
                                    &job,
                                    &batch,
                                    crate::viewers::utc_ms() as i64,
                                )
                                .await;
                        }
                    }
                }
            }
        }
        tokio::select! {_=state.stopping.cancelled()=>break,_=tokio::time::sleep(Duration::from_millis(500))=>{}}
    }
}

// This watcher must never await extraction or embedding network calls.
pub async fn run_knowledge_expiry(state: AppState) {
    loop {
        if state.stopping.is_cancelled() {
            break;
        }
        state.expire_knowledge().await;
        tokio::select! {_=state.stopping.cancelled()=>break,_=tokio::time::sleep(Duration::from_millis(50))=>{}}
    }
}
impl AppState {
    pub(crate) async fn expire_knowledge(&self) {
        if crate::viewers::utc_ms() as i64 >= self.knowledge_deadline.load(Ordering::Acquire) {
            let _gate = self.knowledge_gate.write().await;
            if crate::viewers::utc_ms() as i64 >= self.knowledge_deadline.load(Ordering::Acquire) {
                self.invalidate_knowledge().await;
            }
        }
    }
    pub(crate) async fn enrich_memories(
        &self,
        request: &mut DecisionRequest,
    ) -> Option<KnowledgeStamp> {
        let store = self.memory_store.as_ref()?;
        if request.events.is_empty() {
            return None;
        }
        let ids = store
            .resolve_viewers(&self.config.viewers.scope_id, &request.events)
            .await
            .ok()?;
        let mut query = None;
        if let Some(embedder) = &self.memory_embedder {
            let text = request
                .events
                .iter()
                .filter_map(|e| match &e.kind {
                    meowlive_domain::event::EventKind::Chat { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                if let Ok(Ok(mut batch)) =
                    tokio::time::timeout(Duration::from_millis(300), embedder.embed(&[text])).await
                {
                    if let Some(vector) = batch.vectors.pop() {
                        query = Some(QueryEmbedding {
                            model: batch.model,
                            vector,
                        });
                    }
                }
            }
        }
        let known = ids
            .iter()
            .filter(|id| !id.is_empty())
            .cloned()
            .collect::<Vec<_>>();
        let snapshot = store
            .context(
                &self.config.viewers.scope_id,
                &known,
                query.as_ref(),
                crate::viewers::utc_ms() as i64,
            )
            .await
            .ok()?;
        // UTF-8 bytes are a conservative upper bound for a byte-fallback tokenizer.
        let mut remaining = 1200usize;
        for record in snapshot.records.iter().take(8) {
            let Some(index) = ids.iter().position(|id| id == &record.viewer_id) else {
                continue;
            };
            let Some(event) = request.events.get(index) else {
                continue;
            };
            let text = format!(
                "{}：{} = {}",
                event.viewer, record.candidate.key, record.candidate.value
            );
            if text.len() <= remaining {
                remaining -= text.len();
                request.memory_context.push(text);
            }
        }
        let mut expires_at_ms = snapshot
            .records
            .iter()
            .filter_map(|r| r.expires_at_ms)
            .min()
            .unwrap_or(i64::MAX);
        if self.relationship_store.is_some() {
            use meowlive_application::ports::relationships::{
                EntityKind, RelationConfirmation, RelationKind,
            };
            let mut groups = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for id in &known {
                if seen.insert(id) {
                    let facts = tokio::time::timeout(
                        Duration::from_millis(350),
                        self.relationships(id, 1, 6),
                    )
                    .await;
                    if let Ok(Ok((facts, _))) = facts {
                        groups.push(facts.into_iter());
                    }
                }
            }
            let label=|entity:&meowlive_application::ports::relationships::RelationEntity|->Option<String>{
                if entity.kind==EntityKind::Viewer {
                    ids.iter().position(|id|id==&entity.id).and_then(|i|request.events.get(i)).map(|e|e.viewer.clone())
                }else if matches!(entity.kind,EntityKind::Topic|EntityKind::Activity) {Some(entity.id.clone())}else{None}
            };
            let mut seen = std::collections::HashSet::new();
            let mut added = 0;
            for _ in 0..6 {
                for group in &mut groups {
                    if added >= 3 {
                        break;
                    }
                    let Some(fact) = group.next() else {
                        continue;
                    };
                    if fact.confirmation != RelationConfirmation::Confirmed || !seen.insert(fact.id)
                    {
                        continue;
                    }
                    let (Some(source), Some(target)) = (label(&fact.source), label(&fact.target))
                    else {
                        continue;
                    };
                    let relation = match fact.kind {
                        RelationKind::Mention => "提及",
                        RelationKind::Acquaintance => "认识",
                        RelationKind::Participated => "参与",
                        RelationKind::SharedInterest => "感兴趣",
                        RelationKind::Friend => "确认朋友",
                    };
                    let text = format!("关系事实：{source} → {relation} → {target}");
                    if text.len() <= remaining {
                        remaining -= text.len();
                        request.memory_context.push(text);
                        added += 1;
                        if let Some(expiry) = fact.expires_at_ms {
                            expires_at_ms = expires_at_ms.min(expiry.min(i64::MAX as u64) as i64);
                        }
                    }
                }
            }
        }
        Some(KnowledgeStamp {
            revision: snapshot.revision,
            epoch: self.knowledge_epoch.load(Ordering::Acquire),
            expires_at_ms,
        })
    }
    pub(crate) async fn invalidate_knowledge(&self) {
        self.knowledge_epoch.fetch_add(1, Ordering::AcqRel);
        self.knowledge_deadline.store(i64::MAX, Ordering::Release);
        let mut inner = self.inner.lock().await;
        self.sync_agent(&mut inner);
        inner.agent.invalidate_context(self.now_ms());
        inner.knowledge.clear();
        let generation = inner.queue.stop();
        inner.agent_cancel.cancel();
        inner.agent_cancel = tokio_util::sync::CancellationToken::new();
        inner.generation_cancel.cancel();
        inner.generation_cancel = tokio_util::sync::CancellationToken::new();
        let disconnected = inner.bridge.as_ref().is_some_and(|bridge| {
            bridge
                .control
                .try_send(meowlive_protocol::control::ServerCommand::Stop { generation })
                .is_err()
        });
        if disconnected {
            if let Some(bridge) = inner.bridge.take() {
                bridge.cancel.cancel();
            }
            inner.queue.disconnect();
        }
        self.wake.notify_one();
        self.agent_wake.notify_one();
    }
    pub(crate) async fn knowledge_current(&self, stamp: KnowledgeStamp) -> bool {
        let revision = if let Some(store) = &self.memory_store {
            match store.revision(&self.config.viewers.scope_id).await {
                Ok(value) => value,
                Err(_) => return false,
            }
        } else {
            stamp.revision
        };
        stamp.is_current(
            revision,
            self.knowledge_epoch.load(Ordering::Acquire),
            crate::viewers::utc_ms() as i64,
        )
    }
    pub(crate) async fn speech_knowledge_current(&self, id: &str) -> bool {
        let stamp = self.inner.lock().await.knowledge.get(id).copied();
        match stamp {
            Some(stamp) => self.knowledge_current(stamp).await,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expiry_revision_and_epoch_all_fence_context() {
        let stamp = KnowledgeStamp {
            revision: 3,
            epoch: 7,
            expires_at_ms: 100,
        };
        assert!(stamp.is_current(3, 7, 99));
        assert!(!stamp.is_current(3, 7, 100));
        assert!(!stamp.is_current(4, 7, 99));
        assert!(!stamp.is_current(3, 8, 99));
    }
}
