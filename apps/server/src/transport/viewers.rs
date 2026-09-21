//! 受管理员认证保护的有界观众查询，不向执行端转发档案。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use meowlive_protocol::{
    agent::{EventPayload, GiftMetadataInput},
    viewers as dto,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Page {
    limit: u32,
    offset: u64,
}
impl Default for Page {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
        }
    }
}
impl Page {
    fn validate(&self) -> Result<(), ApiError> {
        if !(1..=100).contains(&self.limit) || self.offset > 1_000_000 {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_page",
                "分页超出有效范围",
            ));
        }
        Ok(())
    }
}

pub async fn viewers(
    State(state): State<AppState>,
    Query(page): Query<Page>,
) -> Result<Json<dto::ViewerPage>, ApiError> {
    page.validate()?;
    let store = state
        .viewer_store
        .as_ref()
        .ok_or_else(crate::viewers::unavailable)?;
    let rows = store
        .list_viewers(&state.config.viewers.scope_id, page.limit, page.offset)
        .await
        .map_err(|_| crate::viewers::unavailable())?;
    Ok(Json(dto::ViewerPage {
        scope_id: state.config.viewers.scope_id.clone(),
        offset: page.offset,
        viewers: rows
            .into_iter()
            .map(|r| dto::ViewerSummary {
                viewer_id: r.viewer_id,
                current_alias: r.current_alias,
                alias_observed_at_ms: r.alias_observed_at_ms,
                identities: r
                    .identities
                    .into_iter()
                    .map(|i| dto::ViewerIdentity {
                        platform: i.platform,
                        namespace: i.namespace,
                        id_kind: i.id_kind,
                        external_id: i.external_id,
                        last_confirmed_at_ms: i.last_confirmed_at_ms,
                    })
                    .collect(),
                aliases: r
                    .aliases
                    .into_iter()
                    .map(|a| dto::ViewerAlias {
                        alias: a.alias,
                        first_seen_at_ms: a.first_seen_at_ms,
                        last_seen_at_ms: a.last_seen_at_ms,
                    })
                    .collect(),
            })
            .collect(),
    }))
}

pub async fn events(
    State(state): State<AppState>,
    Query(page): Query<Page>,
) -> Result<Json<dto::ViewerEventPage>, ApiError> {
    page.validate()?;
    let store = state
        .viewer_store
        .as_ref()
        .ok_or_else(crate::viewers::unavailable)?;
    let rows = store
        .list_events(&state.config.viewers.scope_id, page.limit, page.offset)
        .await
        .map_err(|_| crate::viewers::unavailable())?;
    Ok(Json(dto::ViewerEventPage {
        scope_id: state.config.viewers.scope_id.clone(),
        offset: page.offset,
        unconfirmed_events: state.viewer_gaps.load(std::sync::atomic::Ordering::Relaxed),
        events: rows
            .into_iter()
            .map(|r| dto::PersistedViewerEvent {
                event_id: r.event_id,
                source: r.source,
                session_id: r.session_id,
                viewer_id: r.viewer_id,
                viewer: r.viewer,
                occurred_at_ms: r.occurred_at_ms,
                received_at_ms: r.received_at_ms,
                kind: match r.kind {
                    meowlive_domain::event::EventKind::Chat { text } => EventPayload::Chat { text },
                    meowlive_domain::event::EventKind::Gift { name, count } => {
                        EventPayload::Gift { name, count }
                    }
                    meowlive_domain::event::EventKind::SuperChat {
                        text,
                        amount_cny,
                        start_at_ms,
                        end_at_ms,
                    } => EventPayload::SuperChat {
                        text,
                        amount_cny,
                        start_at_ms,
                        end_at_ms,
                    },
                    meowlive_domain::event::EventKind::RoomEnter => EventPayload::RoomEnter,
                },
                gift_metadata: r.gift_metadata.map(|g| GiftMetadataInput {
                    price: g.price,
                    paid: g.paid,
                    medal_level: g.medal_level,
                    guard_level: g.guard_level,
                }),
            })
            .collect(),
    }))
}
