//! 接收事实、完成回执与管理员操作共享事务性陪伴账本。
use super::*;
use meowlive_application::ports::companionship::*;
use meowlive_domain::affinity::{apply_delta, calendar_day, gift_budget};
use sha2::{Digest, Sha256};
fn invalid() -> ViewerStoreError {
    ViewerStoreError::new("invalid companionship request")
}
fn day(ms: u64, offset: i32) -> Result<i64, ViewerStoreError> {
    calendar_day(ms, offset).ok_or_else(invalid)
}
fn id(value: &str) -> Result<Uuid, ViewerStoreError> {
    Uuid::parse_str(value).map_err(|_| invalid())
}
fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    viewer: Uuid,
) -> Result<(), ViewerStoreError> {
    let exists = sqlx::query_scalar::<_, Uuid>(
        "SELECT c.viewer_id FROM viewer_companionship c JOIN viewers v ON v.scope_id=c.scope_id AND v.id=c.viewer_id WHERE c.scope_id=$1 AND c.viewer_id=$2 AND v.merged_into IS NULL FOR UPDATE OF c",
    )
    .bind(scope)
    .bind(viewer)
    .fetch_optional(&mut **tx)
    .await
    .map_err(database_error)?;
    if exists.is_none() {
        return Err(invalid());
    }
    Ok(())
}
pub(super) async fn observe(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    session: &str,
    viewer: Uuid,
    event: &LiveEvent,
    offset: i32,
) -> Result<(), ViewerStoreError> {
    // Simulator identities are separately namespaced by the event store; no real ledger is created.
    if event.source == "simulator" {
        return Ok(());
    }
    let when = timestamp(event.occurred_at_ms)?;
    let d = day(event.occurred_at_ms, offset)?;
    sqlx::query("INSERT INTO viewer_companionship(scope_id,viewer_id,last_seen_at_ms) VALUES($1,$2,$3) ON CONFLICT(scope_id,viewer_id) DO UPDATE SET last_seen_at_ms=GREATEST(viewer_companionship.last_seen_at_ms,EXCLUDED.last_seen_at_ms)").bind(scope).bind(viewer).bind(when).execute(&mut **tx).await.map_err(database_error)?;
    let fresh=sqlx::query("INSERT INTO viewer_presence(scope_id,viewer_id,day) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(scope).bind(viewer).bind(d).execute(&mut **tx).await.map_err(database_error)?.rows_affected()>0;
    if fresh {
        write_ledger(
            tx,
            scope,
            viewer,
            "presence",
            d,
            1000,
            "daily presence",
            "system",
            when,
            &format!("presence:{viewer}:{d}"),
            "",
            None,
        )
        .await?;
    }
    sqlx::query("INSERT INTO viewer_observed_sessions(scope_id,viewer_id,session_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(scope).bind(viewer).bind(session).execute(&mut **tx).await.map_err(database_error)?;
    if matches!(event.kind, EventKind::Gift { .. }) {
        sqlx::query("INSERT INTO gift_ledger(scope_id,viewer_id,source,event_id,day,occurred_at_ms,payload) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING").bind(scope).bind(viewer).bind(&event.source).bind(&event.id).bind(d).bind(when).bind(sqlx::types::Json(event_payload(event))).execute(&mut **tx).await.map_err(database_error)?;
        if let Some(m) = &event.gift_metadata {
            sqlx::query("UPDATE viewer_companionship SET medal_level=COALESCE($3,medal_level),guard_level=COALESCE($4,guard_level),platform_observed_at_ms=$5 WHERE scope_id=$1 AND viewer_id=$2 AND (platform_observed_at_ms IS NULL OR platform_observed_at_ms<=$5)").bind(scope).bind(viewer).bind(m.medal_level.map(i64::from)).bind(m.guard_level.map(i64::from)).bind(when).execute(&mut **tx).await.map_err(database_error)?;
        }
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
async fn write_ledger(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    viewer: Uuid,
    kind: &str,
    day: i64,
    computed: i32,
    reason: &str,
    actor: &str,
    now: i64,
    key: &str,
    fingerprint: &str,
    reversed: Option<Uuid>,
) -> Result<String, ViewerStoreError> {
    let column = if kind == "presence" {
        "familiarity_milli"
    } else {
        "affinity_milli"
    };
    let current = sqlx::query_scalar::<_, i32>(&format!(
        "SELECT {column} FROM viewer_companionship WHERE scope_id=$1 AND viewer_id=$2 FOR UPDATE"
    ))
    .bind(scope)
    .bind(viewer)
    .fetch_one(&mut **tx)
    .await
    .map_err(database_error)?;
    let (next, applied) = apply_delta(current, computed);
    let ledger = Uuid::new_v4();
    sqlx::query("INSERT INTO affinity_ledger(id,scope_id,viewer_id,kind,day,computed_delta_milli,applied_delta_milli,reason,actor,created_at_ms,request_key,fingerprint,reversed_ledger_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)").bind(ledger).bind(scope).bind(viewer).bind(kind).bind(day).bind(computed).bind(applied).bind(reason).bind(actor).bind(now).bind(key).bind(fingerprint).bind(reversed).execute(&mut **tx).await.map_err(database_error)?;
    sqlx::query(&format!(
        "UPDATE viewer_companionship SET {column}=$3 WHERE scope_id=$1 AND viewer_id=$2"
    ))
    .bind(scope)
    .bind(viewer)
    .bind(next)
    .execute(&mut **tx)
    .await
    .map_err(database_error)?;
    Ok(ledger.to_string())
}
fn admin_validate(scope: &str, key: &str, reason: &str, now: u64) -> Result<(), ViewerStoreError> {
    validate_scope(scope)?;
    validate_key("request key", key, 128)?;
    timestamp(now)?;
    if reason.trim().is_empty() || reason.chars().count() > 1000 {
        return Err(invalid());
    }
    Ok(())
}
async fn previous(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    key: &str,
    fingerprint: &str,
) -> Result<Option<String>, ViewerStoreError> {
    let row = sqlx::query(
        "SELECT id,fingerprint FROM affinity_ledger WHERE scope_id=$1 AND request_key=$2",
    )
    .bind(scope)
    .bind(format!("admin:{key}"))
    .fetch_optional(&mut **tx)
    .await
    .map_err(database_error)?;
    match row {
        None => Ok(None),
        Some(r) => {
            if r.get::<String, _>("fingerprint") != fingerprint {
                return Err(invalid());
            }
            Ok(Some(r.get::<Uuid, _>("id").to_string()))
        }
    }
}
impl CompanionshipStore for PostgresViewerEventStore {
    fn detail<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        limit: u32,
    ) -> ViewerStoreFuture<'a, Option<CompanionshipDetail>> {
        Box::pin(async move {
            validate_scope(scope)?;
            let viewer = id(viewer)?;
            let (limit, _) = page(limit, 0)?;
            let mut tx = self.begin().await?;
            let Some(r)=sqlx::query("SELECT c.*,(SELECT count(*) FROM viewer_presence p WHERE p.scope_id=c.scope_id AND p.viewer_id=c.viewer_id) AS days,(SELECT count(*) FROM viewer_observed_sessions s WHERE s.scope_id=c.scope_id AND s.viewer_id=c.viewer_id) AS sessions FROM viewer_companionship c WHERE scope_id=$1 AND viewer_id=$2 FOR SHARE").bind(scope).bind(viewer).fetch_optional(&mut *tx).await.map_err(database_error)? else{return Ok(None)};
            let mut gifts = Vec::new();
            for g in sqlx::query("SELECT * FROM gift_ledger WHERE scope_id=$1 AND viewer_id=$2 ORDER BY occurred_at_ms DESC,source,event_id LIMIT $3").bind(scope).bind(viewer).bind(limit).fetch_all(&mut *tx).await.map_err(database_error)?{
 let payload:Value=g.try_get::<sqlx::types::Json<Value>,_>("payload").map_err(database_error)?.0;
 let EventKind::Gift{name,count}=payload_kind("gift".into(),&payload)? else{return Err(invalid())};
 gifts.push(GiftLedgerEntry{source:g.get("source"),event_id:g.get("event_id"),name,count,metadata:payload_gift_metadata(&payload)?,value_cents:g.get::<Option<i64>,_>("value_cents").map(|v|v as u64),value_kind:g.get("value_kind"),occurred_at_ms:stored_timestamp(g.get("occurred_at_ms"))?});}
            let mut ledger = Vec::new();
            for l in sqlx::query("SELECT l.*, (l.original_viewer_id IS NULL AND l.kind IN ('manual','gift','exchange') AND NOT EXISTS(SELECT 1 FROM affinity_ledger reversed WHERE reversed.scope_id=l.scope_id AND reversed.reversed_ledger_id=l.id)) AS reversible FROM affinity_ledger l WHERE l.scope_id=$1 AND l.viewer_id=$2 ORDER BY l.created_at_ms DESC,l.id LIMIT $3").bind(scope).bind(viewer).bind(limit).fetch_all(&mut *tx).await.map_err(database_error)?{ledger.push(AffinityLedgerEntry{reversible:l.get("reversible"),ledger_id:l.get::<Uuid,_>("id").to_string(),kind:l.get("kind"),computed_delta_milli:l.get("computed_delta_milli"),applied_delta_milli:l.get("applied_delta_milli"),reason:l.get("reason"),actor:l.get("actor"),created_at_ms:stored_timestamp(l.get("created_at_ms"))?,reversed_ledger_id:l.get::<Option<Uuid>,_>("reversed_ledger_id").map(|v|v.to_string())});}
            let result = CompanionshipDetail {
                viewer_id: viewer.to_string(),
                familiarity_milli: r.get("familiarity_milli"),
                affinity_milli: r.get("affinity_milli"),
                observed_days: r.get::<i64, _>("days") as u64,
                observed_sessions: r.get::<i64, _>("sessions") as u64,
                last_seen_at_ms: stored_timestamp(r.get("last_seen_at_ms"))?,
                medal_level: r.get::<Option<i64>, _>("medal_level").map(|v| v as u32),
                guard_level: r.get::<Option<i64>, _>("guard_level").map(|v| v as u32),
                gifts,
                ledger,
            };
            tx.commit().await.map_err(database_error)?;
            Ok(Some(result))
        })
    }
    fn register_reply<'a>(
        &'a self,
        scope: &'a str,
        speech: &'a str,
        events: &'a [LiveEvent],
        now: u64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async move {
            validate_scope(scope)?;
            validate_key("speech id", speech, 128)?;
            if events.len() > 100 {
                return Err(invalid());
            }
            let mut tx = self.begin().await?;
            super::viewer_merge::lock_scope(&mut tx, scope).await?;
            let keys = events
                .iter()
                .map(|e| (e.source.as_str(), e.id.as_str()))
                .collect::<std::collections::BTreeSet<_>>();
            let encoded = serde_json::to_string(&keys).map_err(|_| invalid())?;
            let new=sqlx::query("INSERT INTO companionship_replies(scope_id,speech_id,generated_at_ms,event_keys) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(scope).bind(speech).bind(timestamp(now)?).bind(&encoded).execute(&mut *tx).await.map_err(database_error)?.rows_affected()>0;
            if !new {
                let original=sqlx::query_scalar::<_,String>("SELECT event_keys FROM companionship_replies WHERE scope_id=$1 AND speech_id=$2").bind(scope).bind(speech).fetch_one(&mut *tx).await.map_err(database_error)?;
                if original != encoded {
                    return Err(invalid());
                }
                return Ok(());
            }
            for e in events {
                let row=sqlx::query("SELECT viewer_id,occurred_at_ms,event_type,payload FROM viewer_events WHERE scope_id=$1 AND source=$2 AND event_id=$3").bind(scope).bind(&e.source).bind(&e.id).fetch_optional(&mut *tx).await.map_err(database_error)?;
                let r = row.ok_or_else(invalid)?;
                {
                    if e.source == "simulator" || r.get::<Option<Uuid>, _>("viewer_id").is_none() {
                        continue;
                    }
                    let payload = r.get::<sqlx::types::Json<Value>, _>("payload").0;
                    let body = if r.get::<String, _>("event_type") == "chat" {
                        payload
                            .get("text")
                            .and_then(Value::as_str)
                            .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
                    } else {
                        None
                    };
                    sqlx::query("INSERT INTO companionship_reply_events(scope_id,speech_id,source,event_id,viewer_id,day,chat_body) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING").bind(scope).bind(speech).bind(&e.source).bind(&e.id).bind(r.get::<Uuid,_>("viewer_id")).bind(day(stored_timestamp(r.get("occurred_at_ms"))?,self.options.calendar_offset_minutes)?).bind(body).execute(&mut *tx).await.map_err(database_error)?;
                }
            }
            tx.commit().await.map_err(database_error)?;
            Ok(())
        })
    }
    fn complete_reply<'a>(
        &'a self,
        scope: &'a str,
        speech: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(async move {
            validate_scope(scope)?;
            validate_key("speech id", speech, 128)?;
            let mut tx = self.begin().await?;
            super::viewer_merge::lock_scope(&mut tx, scope).await?;
            let r=sqlx::query("SELECT completed_at_ms FROM companionship_replies WHERE scope_id=$1 AND speech_id=$2 FOR UPDATE").bind(scope).bind(speech).fetch_optional(&mut *tx).await.map_err(database_error)?.ok_or_else(invalid)?;
            if r.get::<Option<i64>, _>("completed_at_ms").is_some() {
                return Ok(());
            }
            let rows=sqlx::query("SELECT * FROM companionship_reply_events WHERE scope_id=$1 AND speech_id=$2 ORDER BY viewer_id,source,event_id").bind(scope).bind(speech).fetch_all(&mut *tx).await.map_err(database_error)?;
            let mut rewarded = std::collections::BTreeSet::new();
            for r in rows {
                let viewer = r.get::<Uuid, _>("viewer_id");
                lock(&mut tx, scope, viewer).await?;
                let source = r.get::<String, _>("source");
                let event = r.get::<String, _>("event_id");
                let fresh=sqlx::query("INSERT INTO companionship_completed_events(scope_id,source,event_id,viewer_id) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(scope).bind(&source).bind(&event).bind(viewer).execute(&mut *tx).await.map_err(database_error)?.rows_affected()>0;
                let Some(body) = r.get::<Option<String>, _>("chat_body") else {
                    continue;
                };
                if !fresh || body.trim().is_empty() {
                    continue;
                }
                let d = r.get::<i64, _>("day");
                let fresh=sqlx::query("INSERT INTO companionship_daily_chats(scope_id,viewer_id,day,body_hash) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(scope).bind(viewer).bind(d).bind(hash(&body)).execute(&mut *tx).await.map_err(database_error)?.rows_affected()>0;
                if !fresh {
                    continue;
                }
                let used=sqlx::query_scalar::<_,i64>("SELECT COALESCE(sum(computed_delta_milli),0)::bigint FROM affinity_ledger WHERE scope_id=$1 AND viewer_id=$2 AND day=$3 AND kind='exchange'").bind(scope).bind(viewer).bind(d).fetch_one(&mut *tx).await.map_err(database_error)?;
                let computed = if rewarded.insert(viewer) {
                    (2000 - used).clamp(0, 200) as i32
                } else {
                    0
                };
                write_ledger(
                    &mut tx,
                    scope,
                    viewer,
                    "exchange",
                    d,
                    computed,
                    "completed original chat",
                    "system",
                    timestamp(now)?,
                    &format!(
                        "exchange:{}",
                        hash(&serde_json::to_string(&(&source, &event)).map_err(|_| invalid())?)
                    ),
                    "",
                    None,
                )
                .await?;
            }
            sqlx::query("UPDATE companionship_replies SET completed_at_ms=$3 WHERE scope_id=$1 AND speech_id=$2").bind(scope).bind(speech).bind(timestamp(now)?).execute(&mut *tx).await.map_err(database_error)?;
            tx.commit().await.map_err(database_error)?;
            Ok(())
        })
    }
    fn adjust<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        r: &'a AffinityAdjustment,
        now: u64,
    ) -> ViewerStoreFuture<'a, String> {
        Box::pin(async move {
            admin_validate(scope, &r.request_key, &r.reason, now)?;
            if !(-100000..=100000).contains(&r.delta_milli) {
                return Err(invalid());
            }
            let viewer = id(viewer)?;
            let fingerprint = format!("adjust:{viewer}:{r:?}");
            let mut tx = self.begin().await?;
            super::viewer_merge::lock_scope(&mut tx, scope).await?;
            lock(&mut tx, scope, viewer).await?;
            if let Some(old) = previous(&mut tx, scope, &r.request_key, &fingerprint).await? {
                return Ok(old);
            }
            let result = write_ledger(
                &mut tx,
                scope,
                viewer,
                "manual",
                day(now, self.options.calendar_offset_minutes)?,
                r.delta_milli,
                &r.reason,
                "single_admin",
                timestamp(now)?,
                &format!("admin:{}", r.request_key),
                &fingerprint,
                None,
            )
            .await?;
            tx.commit().await.map_err(database_error)?;
            Ok(result)
        })
    }
    fn reverse<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        r: &'a AffinityReversal,
        now: u64,
    ) -> ViewerStoreFuture<'a, String> {
        Box::pin(async move {
            admin_validate(scope, &r.request_key, &r.reason, now)?;
            let viewer = id(viewer)?;
            let target = id(&r.ledger_id)?;
            let fingerprint = format!("reverse:{viewer}:{r:?}");
            let mut tx = self.begin().await?;
            super::viewer_merge::lock_scope(&mut tx, scope).await?;
            lock(&mut tx, scope, viewer).await?;
            if let Some(old) = previous(&mut tx, scope, &r.request_key, &fingerprint).await? {
                return Ok(old);
            }
            let original=sqlx::query("SELECT applied_delta_milli,kind FROM affinity_ledger WHERE scope_id=$1 AND viewer_id=$2 AND id=$3 AND kind IN ('manual','gift','exchange') AND original_viewer_id IS NULL").bind(scope).bind(viewer).bind(target).fetch_optional(&mut *tx).await.map_err(database_error)?.ok_or_else(invalid)?;
            let result = write_ledger(
                &mut tx,
                scope,
                viewer,
                "reversal",
                day(now, self.options.calendar_offset_minutes)?,
                -original.get::<i32, _>("applied_delta_milli"),
                &r.reason,
                "single_admin",
                timestamp(now)?,
                &format!("admin:{}", r.request_key),
                &fingerprint,
                Some(target),
            )
            .await?;
            tx.commit().await.map_err(database_error)?;
            Ok(result)
        })
    }
    fn confirm_gift<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        r: &'a GiftConfirmation,
        now: u64,
    ) -> ViewerStoreFuture<'a, String> {
        Box::pin(async move {
            admin_validate(scope, &r.request_key, &r.reason, now)?;
            if r.value_kind != "confirmed_paid_value" {
                return Err(invalid());
            }
            let value = timestamp(r.value_cents)?;
            let viewer = id(viewer)?;
            let fingerprint = format!("gift:{viewer}:{r:?}");
            let mut tx = self.begin().await?;
            super::viewer_merge::lock_scope(&mut tx, scope).await?;
            lock(&mut tx, scope, viewer).await?;
            if let Some(old) = previous(&mut tx, scope, &r.request_key, &fingerprint).await? {
                return Ok(old);
            }
            let gift=sqlx::query("SELECT day,value_cents,payload FROM gift_ledger WHERE scope_id=$1 AND viewer_id=$2 AND source=$3 AND event_id=$4 FOR UPDATE").bind(scope).bind(viewer).bind(&r.source).bind(&r.event_id).fetch_optional(&mut *tx).await.map_err(database_error)?.ok_or_else(invalid)?;
            if gift.get::<Option<i64>, _>("value_cents").is_some()
                || gift
                    .get::<sqlx::types::Json<Value>, _>("payload")
                    .0
                    .pointer("/gift_metadata/paid")
                    .and_then(Value::as_bool)
                    == Some(false)
            {
                return Err(invalid());
            }
            let d = gift.get::<i64, _>("day");
            let old=sqlx::query_scalar::<_,i64>("SELECT COALESCE(sum(value_cents),0)::bigint FROM gift_ledger WHERE scope_id=$1 AND viewer_id=$2 AND day=$3").bind(scope).bind(viewer).bind(d).fetch_one(&mut *tx).await.map_err(database_error)?;
            let total = old.checked_add(value).ok_or_else(invalid)?;
            sqlx::query("UPDATE gift_ledger SET value_cents=$5,value_kind=$6 WHERE scope_id=$1 AND viewer_id=$2 AND source=$3 AND event_id=$4").bind(scope).bind(viewer).bind(&r.source).bind(&r.event_id).bind(value).bind(&r.value_kind).execute(&mut *tx).await.map_err(database_error)?;
            let result = write_ledger(
                &mut tx,
                scope,
                viewer,
                "gift",
                d,
                gift_budget(total as u64) - gift_budget(old as u64),
                &r.reason,
                "single_admin",
                timestamp(now)?,
                &format!("admin:{}", r.request_key),
                &fingerprint,
                None,
            )
            .await?;
            tx.commit().await.map_err(database_error)?;
            Ok(result)
        })
    }
}
