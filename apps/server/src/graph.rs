//! 图副本异步恢复与 SQL 复核；外部图不可用不阻断回复。
use crate::state::AppState;
use meowlive_application::ports::relationships::*;
use std::{sync::Arc, time::Duration};
pub async fn run_graph(state: AppState) {
    let Some(store) = &state.relationship_store else {
        return;
    };
    if !state.config.graph.enabled {
        return;
    }
    loop {
        if state.stopping.is_cancelled() {
            break;
        }
        let mut graph = state.relationship_graph.read().await.clone();
        if graph.is_none() {
            if let Ok(password) = std::env::var(&state.config.graph.password_env) {
                if let Ok(adapter) = meowlive_adapters::graph::Neo4jRelationshipGraph::connect(
                    &state.config.graph.endpoint,
                    &state.config.graph.user,
                    &password,
                )
                .await
                {
                    graph = Some(Arc::new(adapter) as Arc<dyn RelationshipGraph>);
                    *state.relationship_graph.write().await = graph.clone();
                }
            }
        }
        let mut delay = Duration::from_secs(10);
        if let Some(graph) = graph {
            delay = Duration::from_millis(500);
            if let Ok(Some(job)) = store
                .claim_outbox(&state.config.viewers.scope_id, crate::viewers::utc_ms())
                .await
            {
                if graph.project(&job.projection).await.is_ok() {
                    let _ = store
                        .finish_outbox(
                            &state.config.viewers.scope_id,
                            &job.id,
                            &job.token,
                            crate::viewers::utc_ms(),
                        )
                        .await;
                } else {
                    let _ = store
                        .fail_outbox(
                            &state.config.viewers.scope_id,
                            &job.id,
                            &job.token,
                            crate::viewers::utc_ms(),
                        )
                        .await;
                    *state.relationship_graph.write().await = None;
                }
            }
        }
        tokio::select! {_=state.stopping.cancelled()=>break,_=tokio::time::sleep(delay)=>{}}
    }
}
impl AppState {
    pub(crate) async fn relationships(
        &self,
        viewer: &str,
        depth: u8,
        limit: u32,
    ) -> Result<(Vec<RelationshipFact>, bool), meowlive_application::ports::viewers::ViewerStoreError>
    {
        let store = self.relationship_store.as_ref().ok_or_else(|| {
            meowlive_application::ports::viewers::ViewerStoreError::new("relationships unavailable")
        })?;
        let scope = &self.config.viewers.scope_id;
        // Always fetch canonical facts: a lagging graph is not allowed to hide newly accepted data.
        let mut canonical = store
            .query(scope, viewer, 1, limit, crate::viewers::utc_ms())
            .await?;
        if let Some(graph) = self.relationship_graph.read().await.clone() {
            if let Ok(Ok(refs)) = tokio::time::timeout(
                Duration::from_millis(250),
                graph.neighbors(scope, viewer, depth, limit),
            )
            .await
            {
                if let Ok(facts) = store
                    .validate_graph(scope, &refs, crate::viewers::utc_ms())
                    .await
                {
                    let reachable = store
                        .query(scope, viewer, depth, limit, crate::viewers::utc_ms())
                        .await?;
                    for fact in facts.into_iter().filter(|f| {
                        reachable
                            .iter()
                            .any(|valid| valid.id == f.id && valid.version == f.version)
                    }) {
                        if canonical.len() >= limit as usize {
                            break;
                        }
                        if !canonical.iter().any(|f| f.id == fact.id) {
                            canonical.push(fact);
                        }
                    }
                    return Ok((canonical, false));
                }
            }
        }
        Ok((canonical, true))
    }
}
