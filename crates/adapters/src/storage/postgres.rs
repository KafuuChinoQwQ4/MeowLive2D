//! PostgreSQL 观众身份和直播事件存储适配器。
//! 身份唯一键包含 scope，避免一个部署范围的档案被另一个范围读取。

use meowlive_application::ports::viewers::{
    PersistedEventSummary, StoreEventOutcome, ViewerAliasSummary, ViewerEventStore,
    ViewerIdentitySummary, ViewerStoreError, ViewerStoreFuture, ViewerSummary,
};
use meowlive_domain::event::{
    EventKind, GiftMetadata, LiveEvent, ViewerIdentity, ViewerIdentityKind,
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgPoolOptions, types::Uuid};
use std::time::Duration;

mod companionship;
mod memory;

const MAX_PAGE_SIZE: u32 = 100;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IdentityKey {
    platform: String,
    namespace: String,
    id_kind: String,
    external_id: String,
}

#[derive(Clone, Debug)]
pub struct PostgresViewerStoreOptions {
    pub max_connections: u32,
    pub calendar_offset_minutes: i32,
    pub acquire_timeout: Duration,
    pub statement_timeout: Duration,
    pub lock_timeout: Duration,
}

impl Default for PostgresViewerStoreOptions {
    fn default() -> Self {
        Self {
            max_connections: 5,
            calendar_offset_minutes: 480,
            acquire_timeout: Duration::from_secs(3),
            statement_timeout: Duration::from_secs(5),
            lock_timeout: Duration::from_secs(1),
        }
    }
}

impl PostgresViewerStoreOptions {
    fn validate(&self) -> Result<(), ViewerStoreError> {
        if !(-840..=840).contains(&self.calendar_offset_minutes)
            || self.max_connections == 0
            || self.acquire_timeout.is_zero()
            || self.statement_timeout.is_zero()
            || self.lock_timeout.is_zero()
        {
            return Err(ViewerStoreError::new(
                "PostgreSQL viewer storage options are invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct PostgresViewerEventStore {
    pool: PgPool,
    options: PostgresViewerStoreOptions,
}

impl PostgresViewerEventStore {
    pub async fn connect(database_url: &str) -> Result<Self, ViewerStoreError> {
        Self::connect_with_options(database_url, PostgresViewerStoreOptions::default()).await
    }

    pub async fn connect_with_options(
        database_url: &str,
        options: PostgresViewerStoreOptions,
    ) -> Result<Self, ViewerStoreError> {
        if database_url.trim().is_empty() {
            return Err(ViewerStoreError::new(
                "PostgreSQL viewer database URL is empty",
            ));
        }
        options.validate()?;
        let pool = PgPoolOptions::new()
            .max_connections(options.max_connections)
            .acquire_timeout(options.acquire_timeout)
            .connect(database_url)
            .await
            .map_err(database_error)?;
        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|_| ViewerStoreError::new("PostgreSQL viewer migration failed"))?;
        Ok(Self { pool, options })
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, ViewerStoreError> {
        let mut transaction = self.pool.begin().await.map_err(database_error)?;
        set_transaction_timeouts(&mut transaction, &self.options).await?;
        Ok(transaction)
    }
}

impl ViewerEventStore for PostgresViewerEventStore {
    fn accept_events<'a>(
        &'a self,
        scope_id: &'a str,
        session_id: &'a str,
        events: &'a [LiveEvent],
        received_at_ms: u64,
    ) -> ViewerStoreFuture<'a, Vec<StoreEventOutcome>> {
        Box::pin(async move {
            validate_scope_and_session(scope_id, session_id)?;
            if !(1..=MAX_PAGE_SIZE as usize).contains(&events.len()) {
                return Err(ViewerStoreError::new(format!(
                    "viewer event batch must contain between 1 and {MAX_PAGE_SIZE} events"
                )));
            }
            let received_at_ms = timestamp(received_at_ms)?;
            for event in events {
                event.validate().map_err(ViewerStoreError::new)?;
                timestamp(event.occurred_at_ms)?;
            }

            let mut transaction = self.begin().await?;
            viewer_merge::lock_scope(&mut transaction, scope_id).await?;
            sqlx::query(
                "INSERT INTO live_sessions (scope_id, session_id, first_received_at_ms) \
                 VALUES ($1, $2, $3) ON CONFLICT (scope_id, session_id) DO NOTHING",
            )
            .bind(scope_id)
            .bind(session_id)
            .bind(received_at_ms)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;

            let mut outcomes = vec![None; events.len()];
            let mut new_events = Vec::new();
            let mut ordered_events = events.iter().enumerate().collect::<Vec<_>>();
            ordered_events.sort_by(|(_, first), (_, second)| {
                first
                    .source
                    .cmp(&second.source)
                    .then(first.id.cmp(&second.id))
            });
            for (index, event) in ordered_events {
                let was_inserted = sqlx::query_scalar::<_, String>(
                    "INSERT INTO viewer_events (\
                        scope_id, source, event_id, session_id, viewer_id, viewer_alias, \
                        occurred_at_ms, received_at_ms, event_type, payload\
                    ) VALUES ($1, $2, $3, $4, NULL, $5, $6, $7, $8, $9) \
                    ON CONFLICT (scope_id, source, event_id) DO NOTHING \
                    RETURNING event_id",
                )
                .bind(scope_id)
                .bind(&event.source)
                .bind(&event.id)
                .bind(session_id)
                .bind(&event.viewer)
                .bind(timestamp(event.occurred_at_ms)?)
                .bind(received_at_ms)
                .bind(event_type(&event.kind))
                .bind(sqlx::types::Json(event_payload(event)))
                .fetch_optional(&mut *transaction)
                .await
                .map_err(database_error)?;

                if was_inserted.is_some() {
                    outcomes[index] = Some(StoreEventOutcome {
                        duplicate: false,
                        viewer_id: None,
                    });
                    new_events.push((index, event));
                } else {
                    let viewer_id = sqlx::query_scalar::<_, Option<Uuid>>(
                        "SELECT viewer_id FROM viewer_events \
                         WHERE scope_id = $1 AND source = $2 AND event_id = $3",
                    )
                    .bind(scope_id)
                    .bind(&event.source)
                    .bind(&event.id)
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(database_error)?;
                    outcomes[index] = Some(StoreEventOutcome {
                        duplicate: true,
                        viewer_id: viewer_id.map(|value| value.to_string()),
                    });
                }
            }

            // Resolve new stable identities in a consistent key order so two
            // batches with the same identities do not lock them in opposite order.
            let mut identities =
                std::collections::BTreeMap::<IdentityKey, (&ViewerIdentity, i64)>::new();
            for (_, event) in &new_events {
                if let Some(identity) = &event.viewer_identity {
                    let key = identity_key(event, identity);
                    let occurred_at_ms = timestamp(event.occurred_at_ms)?;
                    identities
                        .entry(key)
                        .and_modify(|(_, latest)| *latest = (*latest).max(occurred_at_ms))
                        .or_insert((identity, occurred_at_ms));
                }
            }
            let mut resolved = std::collections::BTreeMap::<IdentityKey, Uuid>::new();
            for (key, (_, latest_confirmed_at_ms)) in identities {
                let viewer_id = resolve_identity(
                    &mut transaction,
                    scope_id,
                    &key,
                    received_at_ms,
                    latest_confirmed_at_ms,
                )
                .await?;
                resolved.insert(key, viewer_id);
            }
            let viewer_ids = resolved.values().copied().collect::<Vec<_>>();
            sqlx::query("SELECT viewer_id FROM viewer_companionship WHERE scope_id=$1 AND viewer_id=ANY($2) ORDER BY viewer_id FOR UPDATE")
                .bind(scope_id).bind(&viewer_ids).fetch_all(&mut *transaction).await.map_err(database_error)?;
            let mut inserted_viewers = std::collections::BTreeMap::<(String, String), Uuid>::new();
            for (index, event) in new_events {
                let Some(identity) = &event.viewer_identity else {
                    continue;
                };
                let key = identity_key(event, identity);
                let viewer_id = *resolved
                    .get(&key)
                    .ok_or_else(|| ViewerStoreError::new("resolved viewer identity is missing"))?;
                let occurred_at_ms = timestamp(event.occurred_at_ms)?;
                sqlx::query(
                    "UPDATE viewer_events SET viewer_id = $1 \
                     WHERE scope_id = $2 AND source = $3 AND event_id = $4",
                )
                .bind(viewer_id)
                .bind(scope_id)
                .bind(&event.source)
                .bind(&event.id)
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
                upsert_alias(
                    &mut transaction,
                    scope_id,
                    viewer_id,
                    &event.viewer,
                    occurred_at_ms,
                )
                .await?;
                outcomes[index]
                    .as_mut()
                    .ok_or_else(|| ViewerStoreError::new("inserted event outcome is missing"))?
                    .viewer_id = Some(viewer_id.to_string());
                companionship::observe(
                    &mut transaction,
                    scope_id,
                    session_id,
                    viewer_id,
                    event,
                    self.options.calendar_offset_minutes,
                )
                .await?;
                memory::enqueue(
                    &mut transaction,
                    scope_id,
                    &event.source,
                    &event.id,
                    received_at_ms,
                    self.options.calendar_offset_minutes,
                )
                .await?;
                inserted_viewers.insert((event.source.clone(), event.id.clone()), viewer_id);
            }
            for (index, event) in events.iter().enumerate() {
                if outcomes[index]
                    .as_ref()
                    .is_some_and(|outcome| outcome.duplicate)
                    && let Some(viewer_id) =
                        inserted_viewers.get(&(event.source.clone(), event.id.clone()))
                {
                    outcomes[index]
                        .as_mut()
                        .ok_or_else(|| ViewerStoreError::new("duplicate event outcome is missing"))?
                        .viewer_id = Some(viewer_id.to_string());
                }
            }
            let outcomes = outcomes
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| ViewerStoreError::new("viewer event outcome is missing"))?;
            transaction.commit().await.map_err(database_error)?;
            Ok(outcomes)
        })
    }

    fn list_viewers<'a>(
        &'a self,
        scope_id: &'a str,
        limit: u32,
        offset: u64,
    ) -> ViewerStoreFuture<'a, Vec<ViewerSummary>> {
        Box::pin(async move {
            validate_scope(scope_id)?;
            let (limit, offset) = page(limit, offset)?;
            let mut transaction = self.begin().await?;
            let rows = sqlx::query(
                "SELECT id, current_alias, alias_observed_at_ms \
                 FROM viewers WHERE scope_id = $1 AND merged_into IS NULL \
                 ORDER BY alias_observed_at_ms DESC NULLS LAST, id LIMIT $2 OFFSET $3",
            )
            .bind(scope_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *transaction)
            .await
            .map_err(database_error)?;
            let viewer_ids = rows
                .iter()
                .map(|row| row.try_get::<Uuid, _>("id").map_err(database_error))
                .collect::<Result<Vec<_>, _>>()?;
            let identity_rows = if viewer_ids.is_empty() {
                Vec::new()
            } else {
                sqlx::query(
                    "SELECT identity.viewer_id, identity.platform, identity.namespace, identity.id_kind, \
                            identity.external_id, identity.last_confirmed_at_ms \
                     FROM unnest($2::uuid[]) requested(viewer_id) \
                     CROSS JOIN LATERAL (\
                        SELECT viewer_id, platform, namespace, id_kind, external_id, last_confirmed_at_ms \
                        FROM viewer_identities \
                        WHERE scope_id = $1 AND viewer_id = requested.viewer_id \
                        ORDER BY last_confirmed_at_ms DESC, platform, namespace, id_kind, external_id \
                        LIMIT 100\
                     ) identity",
                )
                .bind(scope_id)
                .bind(&viewer_ids)
                .fetch_all(&mut *transaction)
                .await
                .map_err(database_error)?
            };
            let alias_rows = if viewer_ids.is_empty() {
                Vec::new()
            } else {
                sqlx::query(
                    "SELECT alias.viewer_id, alias.alias, alias.first_seen_at_ms, alias.last_seen_at_ms \
                     FROM unnest($2::uuid[]) requested(viewer_id) \
                     CROSS JOIN LATERAL (\
                        SELECT viewer_id, alias, first_seen_at_ms, last_seen_at_ms \
                        FROM viewer_aliases \
                        WHERE scope_id = $1 AND viewer_id = requested.viewer_id \
                        ORDER BY last_seen_at_ms DESC, alias \
                        LIMIT 100\
                     ) alias",
                )
                .bind(scope_id)
                .bind(&viewer_ids)
                .fetch_all(&mut *transaction)
                .await
                .map_err(database_error)?
            };
            let mut identities =
                std::collections::BTreeMap::<Uuid, Vec<ViewerIdentitySummary>>::new();
            for row in identity_rows {
                let viewer_id = row.try_get("viewer_id").map_err(database_error)?;
                identities
                    .entry(viewer_id)
                    .or_default()
                    .push(ViewerIdentitySummary {
                        platform: row.try_get("platform").map_err(database_error)?,
                        namespace: row.try_get("namespace").map_err(database_error)?,
                        id_kind: row.try_get("id_kind").map_err(database_error)?,
                        external_id: row.try_get("external_id").map_err(database_error)?,
                        last_confirmed_at_ms: stored_timestamp(
                            row.try_get("last_confirmed_at_ms")
                                .map_err(database_error)?,
                        )?,
                    });
            }
            let mut aliases = std::collections::BTreeMap::<Uuid, Vec<ViewerAliasSummary>>::new();
            for row in alias_rows {
                let viewer_id = row.try_get("viewer_id").map_err(database_error)?;
                aliases
                    .entry(viewer_id)
                    .or_default()
                    .push(ViewerAliasSummary {
                        alias: row.try_get("alias").map_err(database_error)?,
                        first_seen_at_ms: stored_timestamp(
                            row.try_get("first_seen_at_ms").map_err(database_error)?,
                        )?,
                        last_seen_at_ms: stored_timestamp(
                            row.try_get("last_seen_at_ms").map_err(database_error)?,
                        )?,
                    });
            }
            let summaries = rows
                .into_iter()
                .map(|row| {
                    let id: Uuid = row.try_get("id").map_err(database_error)?;
                    Ok(ViewerSummary {
                        viewer_id: id.to_string(),
                        current_alias: row.try_get("current_alias").map_err(database_error)?,
                        alias_observed_at_ms: row
                            .try_get::<Option<i64>, _>("alias_observed_at_ms")
                            .map_err(database_error)?
                            .map(stored_timestamp)
                            .transpose()?,
                        aliases: aliases.remove(&id).unwrap_or_default(),
                        identities: identities.remove(&id).unwrap_or_default(),
                    })
                })
                .collect::<Result<Vec<_>, ViewerStoreError>>()?;
            transaction.commit().await.map_err(database_error)?;
            Ok(summaries)
        })
    }

    fn list_events<'a>(
        &'a self,
        scope_id: &'a str,
        limit: u32,
        offset: u64,
    ) -> ViewerStoreFuture<'a, Vec<PersistedEventSummary>> {
        Box::pin(async move {
            validate_scope(scope_id)?;
            let (limit, offset) = page(limit, offset)?;
            let mut transaction = self.begin().await?;
            let rows = sqlx::query(
                "SELECT event_id, source, session_id, viewer_id, viewer_alias, occurred_at_ms, \
                        received_at_ms, event_type, payload \
                 FROM viewer_events WHERE scope_id = $1 \
                 ORDER BY received_at_ms DESC, source, event_id LIMIT $2 OFFSET $3",
            )
            .bind(scope_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *transaction)
            .await
            .map_err(database_error)?;
            let events = rows
                .into_iter()
                .map(|row| {
                    let payload: sqlx::types::Json<Value> =
                        row.try_get("payload").map_err(database_error)?;
                    Ok(PersistedEventSummary {
                        event_id: row.try_get("event_id").map_err(database_error)?,
                        source: row.try_get("source").map_err(database_error)?,
                        session_id: row.try_get("session_id").map_err(database_error)?,
                        viewer_id: row
                            .try_get::<Option<Uuid>, _>("viewer_id")
                            .map_err(database_error)?
                            .map(|value| value.to_string()),
                        viewer: row.try_get("viewer_alias").map_err(database_error)?,
                        occurred_at_ms: stored_timestamp(
                            row.try_get("occurred_at_ms").map_err(database_error)?,
                        )?,
                        received_at_ms: stored_timestamp(
                            row.try_get("received_at_ms").map_err(database_error)?,
                        )?,
                        kind: payload_kind(
                            row.try_get("event_type").map_err(database_error)?,
                            &payload.0,
                        )?,
                        gift_metadata: payload_gift_metadata(&payload.0)?,
                    })
                })
                .collect::<Result<Vec<_>, ViewerStoreError>>()?;
            transaction.commit().await.map_err(database_error)?;
            Ok(events)
        })
    }
}

async fn set_transaction_timeouts(
    transaction: &mut Transaction<'_, Postgres>,
    options: &PostgresViewerStoreOptions,
) -> Result<(), ViewerStoreError> {
    for (name, timeout) in [
        ("statement_timeout", options.statement_timeout),
        ("lock_timeout", options.lock_timeout),
    ] {
        sqlx::query("SELECT set_config($1, $2, true)")
            .bind(name)
            .bind(format!("{}ms", timeout.as_millis()))
            .execute(&mut **transaction)
            .await
            .map_err(database_error)?;
    }
    Ok(())
}

fn validate_scope_and_session(scope_id: &str, session_id: &str) -> Result<(), ViewerStoreError> {
    validate_scope(scope_id)?;
    validate_key("session id", session_id, 128)
}

fn validate_scope(scope_id: &str) -> Result<(), ViewerStoreError> {
    validate_key("scope id", scope_id, 128)
}

fn validate_key(label: &str, value: &str, maximum: usize) -> Result<(), ViewerStoreError> {
    if value.trim().is_empty()
        || value.chars().count() > maximum
        || !value.bytes().all(|byte| (b' '..=b'~').contains(&byte))
    {
        return Err(ViewerStoreError::new(format!("{label} is invalid")));
    }
    Ok(())
}

fn page(limit: u32, offset: u64) -> Result<(i64, i64), ViewerStoreError> {
    if !(1..=MAX_PAGE_SIZE).contains(&limit) {
        return Err(ViewerStoreError::new(format!(
            "viewer query limit must be between 1 and {MAX_PAGE_SIZE}"
        )));
    }
    Ok((i64::from(limit), timestamp(offset)?))
}

fn timestamp(value: u64) -> Result<i64, ViewerStoreError> {
    i64::try_from(value)
        .map_err(|_| ViewerStoreError::new("UTC millisecond timestamp is too large"))
}

fn stored_timestamp(value: i64) -> Result<u64, ViewerStoreError> {
    u64::try_from(value)
        .map_err(|_| ViewerStoreError::new("stored UTC millisecond timestamp is invalid"))
}

fn event_type(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Chat { .. } => "chat",
        EventKind::Gift { .. } => "gift",
        EventKind::SuperChat { .. } => "super_chat",
        EventKind::RoomEnter => "room_enter",
    }
}

fn event_payload(event: &LiveEvent) -> Value {
    let mut payload = match &event.kind {
        EventKind::Chat { text } => json!({ "text": text }),
        EventKind::Gift { name, count } => json!({ "name": name, "count": count }),
        EventKind::SuperChat {
            text,
            amount_cny,
            start_at_ms,
            end_at_ms,
        } => json!({
            "text": text,
            "amount_cny": amount_cny,
            "start_at_ms": start_at_ms,
            "end_at_ms": end_at_ms
        }),
        EventKind::RoomEnter => json!({}),
    };
    if let Some(identity) = &event.viewer_identity {
        payload["viewer_identity"] = json!({
            "namespace": identity.namespace,
            "id_kind": identity_kind(&identity.kind),
            "external_id": identity.external_id,
        });
    }
    if let Some(metadata) = &event.gift_metadata {
        payload["gift_metadata"] = json!({
            "price": metadata.price,
            "paid": metadata.paid,
            "medal_level": metadata.medal_level,
            "guard_level": metadata.guard_level,
        });
    }
    payload
}

fn payload_kind(event_type: String, payload: &Value) -> Result<EventKind, ViewerStoreError> {
    match event_type.as_str() {
        "chat" => payload
            .get("text")
            .and_then(Value::as_str)
            .map(|text| EventKind::Chat { text: text.into() })
            .ok_or_else(|| ViewerStoreError::new("stored chat payload is invalid")),
        "gift" => match (
            payload.get("name").and_then(Value::as_str),
            payload.get("count").and_then(Value::as_u64),
        ) {
            (Some(name), Some(count)) => u32::try_from(count)
                .map(|count| EventKind::Gift {
                    name: name.into(),
                    count,
                })
                .map_err(|_| ViewerStoreError::new("stored gift payload is invalid")),
            _ => Err(ViewerStoreError::new("stored gift payload is invalid")),
        },
        "super_chat" => {
            let invalid = || ViewerStoreError::new("stored super chat payload is invalid");
            let text = payload
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
            let amount_cny = payload
                .get("amount_cny")
                .and_then(Value::as_u64)
                .and_then(|amount| u32::try_from(amount).ok())
                .ok_or_else(invalid)?;
            let start_at_ms = payload
                .get("start_at_ms")
                .and_then(Value::as_u64)
                .ok_or_else(invalid)?;
            let end_at_ms = payload
                .get("end_at_ms")
                .and_then(Value::as_u64)
                .ok_or_else(invalid)?;
            Ok(EventKind::SuperChat {
                text: text.into(),
                amount_cny,
                start_at_ms,
                end_at_ms,
            })
        }
        "room_enter" => Ok(EventKind::RoomEnter),
        _ => Err(ViewerStoreError::new("stored event type is invalid")),
    }
}

fn payload_gift_metadata(payload: &Value) -> Result<Option<GiftMetadata>, ViewerStoreError> {
    let Some(metadata) = payload.get("gift_metadata") else {
        return Ok(None);
    };
    if metadata.is_null() {
        return Ok(None);
    }
    let object = metadata
        .as_object()
        .ok_or_else(|| ViewerStoreError::new("stored gift metadata is invalid"))?;
    let number = |key: &str| -> Result<Option<u64>, ViewerStoreError> {
        match object.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value
                .as_u64()
                .map(Some)
                .ok_or_else(|| ViewerStoreError::new("stored gift metadata is invalid")),
        }
    };
    let level = |key: &str| -> Result<Option<u32>, ViewerStoreError> {
        number(key)?
            .map(u32::try_from)
            .transpose()
            .map_err(|_| ViewerStoreError::new("stored gift metadata is invalid"))
    };
    let paid = match object.get("paid") {
        None | Some(Value::Null) => None,
        Some(value) => value
            .as_bool()
            .ok_or_else(|| ViewerStoreError::new("stored gift metadata is invalid"))
            .map(Some)?,
    };
    Ok(Some(GiftMetadata {
        price: number("price")?,
        paid,
        medal_level: level("medal_level")?,
        guard_level: level("guard_level")?,
    }))
}

fn identity_key(event: &LiveEvent, identity: &ViewerIdentity) -> IdentityKey {
    IdentityKey {
        platform: event.source.clone(),
        namespace: identity.namespace.clone(),
        id_kind: identity_kind(&identity.kind).into(),
        external_id: identity.external_id.clone(),
    }
}

fn identity_kind(kind: &ViewerIdentityKind) -> &'static str {
    match kind {
        ViewerIdentityKind::OpenId => "open_id",
        ViewerIdentityKind::Uid => "uid",
    }
}

async fn resolve_identity(
    transaction: &mut Transaction<'_, Postgres>,
    scope_id: &str,
    key: &IdentityKey,
    created_at_ms: i64,
    last_confirmed_at_ms: i64,
) -> Result<Uuid, ViewerStoreError> {
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT viewer_id FROM viewer_identities \
         WHERE scope_id = $1 AND platform = $2 AND namespace = $3 \
           AND id_kind = $4 AND external_id = $5 FOR UPDATE",
    )
    .bind(scope_id)
    .bind(&key.platform)
    .bind(&key.namespace)
    .bind(&key.id_kind)
    .bind(&key.external_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(database_error)?;
    if let Some(viewer_id) = existing {
        sqlx::query(
            "UPDATE viewer_identities SET last_confirmed_at_ms = GREATEST(last_confirmed_at_ms, $1) \
             WHERE scope_id = $2 AND platform = $3 AND namespace = $4 \
               AND id_kind = $5 AND external_id = $6",
        )
        .bind(last_confirmed_at_ms)
        .bind(scope_id)
        .bind(&key.platform)
        .bind(&key.namespace)
        .bind(&key.id_kind)
        .bind(&key.external_id)
        .execute(&mut **transaction)
        .await
        .map_err(database_error)?;
        return Ok(viewer_id);
    }

    let candidate_id = Uuid::new_v4();
    sqlx::query("INSERT INTO viewers (id, scope_id, created_at_ms) VALUES ($1, $2, $3)")
        .bind(candidate_id)
        .bind(scope_id)
        .bind(created_at_ms)
        .execute(&mut **transaction)
        .await
        .map_err(database_error)?;
    let viewer_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO viewer_identities (\
            scope_id, platform, namespace, id_kind, external_id, viewer_id, last_confirmed_at_ms\
         ) VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (scope_id, platform, namespace, id_kind, external_id) \
         DO UPDATE SET last_confirmed_at_ms = GREATEST(\
            viewer_identities.last_confirmed_at_ms, EXCLUDED.last_confirmed_at_ms\
         ) RETURNING viewer_id",
    )
    .bind(scope_id)
    .bind(&key.platform)
    .bind(&key.namespace)
    .bind(&key.id_kind)
    .bind(&key.external_id)
    .bind(candidate_id)
    .bind(last_confirmed_at_ms)
    .fetch_one(&mut **transaction)
    .await
    .map_err(database_error)?;
    if viewer_id != candidate_id {
        sqlx::query("DELETE FROM viewers WHERE scope_id = $1 AND id = $2")
            .bind(scope_id)
            .bind(candidate_id)
            .execute(&mut **transaction)
            .await
            .map_err(database_error)?;
    }
    Ok(viewer_id)
}

async fn upsert_alias(
    transaction: &mut Transaction<'_, Postgres>,
    scope_id: &str,
    viewer_id: Uuid,
    alias: &str,
    occurred_at_ms: i64,
) -> Result<(), ViewerStoreError> {
    sqlx::query(
        "INSERT INTO viewer_aliases (\
            scope_id, viewer_id, alias, first_seen_at_ms, last_seen_at_ms\
         ) VALUES ($1, $2, $3, $4, $4) \
         ON CONFLICT (scope_id, viewer_id, alias) DO UPDATE \
         SET first_seen_at_ms = LEAST(viewer_aliases.first_seen_at_ms, EXCLUDED.first_seen_at_ms), \
             last_seen_at_ms = GREATEST(viewer_aliases.last_seen_at_ms, EXCLUDED.last_seen_at_ms)",
    )
    .bind(scope_id)
    .bind(viewer_id)
    .bind(alias)
    .bind(occurred_at_ms)
    .execute(&mut **transaction)
    .await
    .map_err(database_error)?;
    sqlx::query(
        "UPDATE viewers SET current_alias = $1, alias_observed_at_ms = $2 \
         WHERE scope_id = $3 AND id = $4 \
           AND (alias_observed_at_ms IS NULL OR alias_observed_at_ms <= $2)",
    )
    .bind(alias)
    .bind(occurred_at_ms)
    .bind(scope_id)
    .bind(viewer_id)
    .execute(&mut **transaction)
    .await
    .map_err(database_error)?;
    Ok(())
}

fn database_error(_error: sqlx::Error) -> ViewerStoreError {
    ViewerStoreError::new("PostgreSQL viewer storage operation failed")
}

mod relationships;

mod viewer_merge;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_super_chat_retains_amount_text_and_active_period() {
        let kind = payload_kind(
            "super_chat".into(),
            &json!({
                "text": "请介绍今天的主题",
                "amount_cny": 30,
                "start_at_ms": 1_700_000_000_000_u64,
                "end_at_ms": 1_700_000_060_000_u64
            }),
        );
        assert_eq!(
            kind.unwrap(),
            EventKind::SuperChat {
                text: "请介绍今天的主题".into(),
                amount_cny: 30,
                start_at_ms: 1_700_000_000_000,
                end_at_ms: 1_700_000_060_000,
            }
        );
    }

    #[test]
    fn stored_room_entry_does_not_require_chat_or_gift_fields() {
        let kind = payload_kind("room_enter".into(), &json!({}));
        assert_eq!(kind.unwrap(), EventKind::RoomEnter);
    }

    #[test]
    fn event_payload_keeps_sc_money_and_times_separate_from_gifts() {
        let mut event = LiveEvent {
            id: "sc-1".into(),
            source: "bilibili".into(),
            viewer: "Alice".into(),
            viewer_identity: None,
            occurred_at_ms: 1_700_000_000_000,
            gift_metadata: None,
            kind: EventKind::SuperChat {
                text: "请介绍今天的主题".into(),
                amount_cny: 30,
                start_at_ms: 1_700_000_000_000,
                end_at_ms: 1_700_000_060_000,
            },
        };
        assert_eq!(event_type(&event.kind), "super_chat");
        assert_eq!(
            event_payload(&event),
            json!({
                "text": "请介绍今天的主题",
                "amount_cny": 30,
                "start_at_ms": 1_700_000_000_000_u64,
                "end_at_ms": 1_700_000_060_000_u64,
            })
        );
        event.kind = EventKind::RoomEnter;
        assert_eq!(event_type(&event.kind), "room_enter");
        assert_eq!(event_payload(&event), json!({}));
    }

    #[test]
    fn stored_super_chat_rejects_missing_or_non_integer_fields() {
        let base = json!({
            "text": "hello",
            "amount_cny": 30,
            "start_at_ms": 1_700_000_000_000_u64,
            "end_at_ms": 1_700_000_060_000_u64,
        });
        for field in ["text", "amount_cny", "start_at_ms", "end_at_ms"] {
            let mut payload = base.clone();
            payload.as_object_mut().unwrap().remove(field);
            assert!(payload_kind("super_chat".into(), &payload).is_err());
        }
        for (field, value) in [
            ("amount_cny", json!(-1)),
            ("amount_cny", json!(30.5)),
            ("amount_cny", json!(u64::MAX)),
            ("start_at_ms", json!(-1)),
            ("end_at_ms", json!("later")),
        ] {
            let mut payload = base.clone();
            payload[field] = value;
            assert!(payload_kind("super_chat".into(), &payload).is_err());
        }
    }
}
