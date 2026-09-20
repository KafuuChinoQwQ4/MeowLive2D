//! 范围锁串行化关系事实、审计和图outbox；SQL始终是有效事实的权威。
use super::*;
use meowlive_application::ports::relationships::*;
pub(in crate::storage::postgres) mod derived;
mod queue;
fn invalid() -> ViewerStoreError {
    ViewerStoreError::new("invalid or stale relationship operation")
}
fn id(s: &str) -> Result<Uuid, ViewerStoreError> {
    Uuid::parse_str(s).map_err(|_| invalid())
}
async fn lock(tx: &mut Transaction<'_, Postgres>, scope: &str) -> Result<(), ViewerStoreError> {
    validate_scope(scope)?;
    sqlx::query("INSERT INTO memory_scopes(scope_id) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(scope)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    sqlx::query("SELECT revision FROM memory_scopes WHERE scope_id=$1 FOR UPDATE")
        .bind(scope)
        .fetch_one(&mut **tx)
        .await
        .map_err(database_error)?;
    Ok(())
}
fn entity_kind(k: EntityKind) -> &'static str {
    match k {
        EntityKind::Viewer => "viewer",
        EntityKind::Topic => "topic",
        EntityKind::Activity => "activity",
        EntityKind::Unresolved => "unresolved",
    }
}
fn parse_entity(s: &str) -> Result<EntityKind, ViewerStoreError> {
    Ok(match s {
        "viewer" => EntityKind::Viewer,
        "topic" => EntityKind::Topic,
        "activity" => EntityKind::Activity,
        "unresolved" => EntityKind::Unresolved,
        _ => return Err(invalid()),
    })
}
fn relation_kind(k: RelationKind) -> &'static str {
    match k {
        RelationKind::Mention => "mention",
        RelationKind::Acquaintance => "acquaintance",
        RelationKind::Participated => "participated",
        RelationKind::SharedInterest => "shared_interest",
        RelationKind::Friend => "friend",
    }
}
fn parse_kind(s: &str) -> Result<RelationKind, ViewerStoreError> {
    Ok(match s {
        "mention" => RelationKind::Mention,
        "acquaintance" => RelationKind::Acquaintance,
        "participated" => RelationKind::Participated,
        "shared_interest" => RelationKind::SharedInterest,
        "friend" => RelationKind::Friend,
        _ => return Err(invalid()),
    })
}
fn encode(f: &RelationshipFact) -> Value {
    json!({"id":f.id,"version":f.version,"source_kind":entity_kind(f.source.kind),"source_id":f.source.id,"target_kind":entity_kind(f.target.kind),"target_id":f.target.id,"kind":relation_kind(f.kind),"confirmation":if f.confirmation==RelationConfirmation::Confirmed{"confirmed"}else{"claimed"},"evidence":f.evidence.iter().map(|e|json!({"source":e.source,"event_id":e.event_id,"quote":e.quote})).collect::<Vec<_>>(),"expires_at_ms":f.expires_at_ms,"deleted":f.deleted})
}
fn decode(v: &Value) -> Result<RelationshipFact, ViewerStoreError> {
    let s = |k| v.get(k).and_then(Value::as_str).ok_or_else(invalid);
    let evidence = v
        .get("evidence")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?
        .iter()
        .map(|e| {
            Ok(RelationEvidence {
                source: e
                    .get("source")
                    .and_then(Value::as_str)
                    .ok_or_else(invalid)?
                    .into(),
                event_id: e
                    .get("event_id")
                    .and_then(Value::as_str)
                    .ok_or_else(invalid)?
                    .into(),
                quote: e
                    .get("quote")
                    .and_then(Value::as_str)
                    .ok_or_else(invalid)?
                    .into(),
            })
        })
        .collect::<Result<Vec<_>, ViewerStoreError>>()?;
    Ok(RelationshipFact {
        id: s("id")?.into(),
        version: v
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(invalid)?,
        source: RelationEntity {
            kind: parse_entity(s("source_kind")?)?,
            id: s("source_id")?.into(),
        },
        target: RelationEntity {
            kind: parse_entity(s("target_kind")?)?,
            id: s("target_id")?.into(),
        },
        kind: parse_kind(s("kind")?)?,
        confirmation: if s("confirmation")? == "confirmed" {
            RelationConfirmation::Confirmed
        } else {
            RelationConfirmation::Claimed
        },
        evidence,
        expires_at_ms: v.get("expires_at_ms").and_then(Value::as_u64),
        deleted: v
            .get("deleted")
            .and_then(Value::as_bool)
            .ok_or_else(invalid)?,
    })
}
async fn fetch(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    fact: Uuid,
) -> Result<RelationshipFact, ViewerStoreError> {
    let v = sqlx::query_scalar::<_, sqlx::types::Json<Value>>(
        "SELECT to_jsonb(f) FROM relationship_facts f WHERE scope_id=$1 AND id=$2",
    )
    .bind(scope)
    .bind(fact)
    .fetch_optional(&mut **tx)
    .await
    .map_err(database_error)?
    .ok_or_else(invalid)?;
    decode(&v.0)
}
fn admin(scope: &str, key: &str, reason: &str, now: u64) -> Result<(), ViewerStoreError> {
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
) -> Result<Option<Value>, ViewerStoreError> {
    let row = sqlx::query(
        "SELECT fingerprint,result FROM relationship_audit WHERE scope_id=$1 AND request_key=$2",
    )
    .bind(scope)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(database_error)?;
    if let Some(r) = row {
        if r.get::<String, _>("fingerprint") != fingerprint {
            return Err(invalid());
        }
        return Ok(Some(r.get::<sqlx::types::Json<Value>, _>("result").0));
    }
    Ok(None)
}
#[allow(clippy::too_many_arguments)]
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    key: &str,
    fingerprint: &str,
    fact: Option<Uuid>,
    reason: &str,
    now: i64,
    result: Value,
) -> Result<(), ViewerStoreError> {
    sqlx::query("INSERT INTO relationship_audit(scope_id,request_key,fingerprint,fact_id,reason,created_at_ms,result) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(scope).bind(key).bind(fingerprint).bind(fact).bind(reason).bind(now).bind(sqlx::types::Json(result)).execute(&mut **tx).await.map_err(database_error)?;
    Ok(())
}
async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    f: &RelationshipFact,
    now: i64,
) -> Result<(), ViewerStoreError> {
    sqlx::query("INSERT INTO relationship_outbox(scope_id,fact_id,version,available_at_ms,created_at_ms) VALUES($1,$2,$3,$4,$4) ON CONFLICT(scope_id,fact_id) DO UPDATE SET version=EXCLUDED.version,state='pending',attempts=0,available_at_ms=EXCLUDED.available_at_ms,created_at_ms=EXCLUDED.created_at_ms,token=NULL,lease_until_ms=NULL").bind(scope).bind(id(&f.id)?).bind(timestamp(f.version)?).bind(now).execute(&mut **tx).await.map_err(database_error)?;
    Ok(())
}
async fn bump(tx: &mut Transaction<'_, Postgres>, scope: &str) -> Result<(), ViewerStoreError> {
    sqlx::query("UPDATE memory_scopes SET revision=revision+1 WHERE scope_id=$1")
        .bind(scope)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    Ok(())
}
// Retrieval carries provenance deadlines into the server knowledge fence; stored/admin facts remain unchanged.
async fn effective_expiry(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    mut fact: RelationshipFact,
) -> Result<RelationshipFact, ViewerStoreError> {
    let expiry=sqlx::query_scalar::<_,Option<i64>>("SELECT min(m.expires_at_ms) FROM relationship_facts f CROSS JOIN LATERAL jsonb_array_elements(f.evidence) proof(item) JOIN memory_evidence me ON me.scope_id=f.scope_id AND me.source=proof.item->>'source' AND me.event_id=proof.item->>'event_id' JOIN memories m ON m.scope_id=me.scope_id AND m.id=me.memory_id WHERE f.scope_id=$1 AND f.id=$2").bind(scope).bind(id(&fact.id)?).fetch_one(&mut **tx).await.map_err(database_error)?;
    if let Some(expiry) = expiry {
        let expiry = stored_timestamp(expiry)?;
        fact.expires_at_ms = Some(fact.expires_at_ms.map_or(expiry, |own| own.min(expiry)));
    }
    Ok(fact)
}
impl RelationshipStore for PostgresViewerEventStore {
    fn create<'a>(
        &'a self,
        scope: &'a str,
        input: &'a RelationInput,
        now: u64,
    ) -> ViewerStoreFuture<'a, RelationshipFact> {
        Box::pin(async move {
            admin(scope, &input.request_key, &input.reason, now)?;
            if input.evidence.len() > 8
                || (!input.admin_confirmed && input.evidence.is_empty())
                || input.source == input.target
            {
                return Err(invalid());
            }
            if let Some(end) = input.expires_at_ms {
                timestamp(end)?;
                if end <= now {
                    return Err(invalid());
                }
            }
            if input.source.kind != EntityKind::Viewer
                || input.target.kind == EntityKind::Unresolved
                    && input.kind != RelationKind::Mention
            {
                return Err(invalid());
            }
            if matches!(
                input.kind,
                RelationKind::Friend | RelationKind::Acquaintance
            ) && input.target.kind != EntityKind::Viewer
            {
                return Err(invalid());
            }
            if input.kind == RelationKind::Participated && input.target.kind != EntityKind::Activity
                || input.kind == RelationKind::SharedInterest
                    && input.target.kind != EntityKind::Topic
            {
                return Err(invalid());
            }
            let mut tx = self.begin().await?;
            lock(&mut tx, scope).await?;
            let fingerprint = format!("create:{input:?}");
            if let Some(old) = previous(&mut tx, scope, &input.request_key, &fingerprint).await? {
                return decode(&old);
            }
            for endpoint in [&input.source, &input.target] {
                if endpoint.id.trim().is_empty()
                    || endpoint.id.chars().count() > 128
                    || endpoint.id.chars().any(char::is_control)
                {
                    return Err(invalid());
                }
                if endpoint.kind == EntityKind::Viewer {
                    let exists = sqlx::query_scalar::<_, Uuid>(
                        "SELECT id FROM viewers WHERE scope_id=$1 AND id=$2 AND merged_into IS NULL",
                    )
                    .bind(scope)
                    .bind(id(&endpoint.id)?)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(database_error)?;
                    if exists.is_none() {
                        return Err(invalid());
                    }
                }
            }
            let mut authors = std::collections::BTreeSet::new();
            let mut unique = std::collections::BTreeSet::new();
            for e in &input.evidence {
                if e.quote.trim().is_empty()
                    || e.quote.chars().count() > 300
                    || !unique.insert((&e.source, &e.event_id))
                {
                    return Err(invalid());
                }
                let r=sqlx::query("SELECT viewer_id,payload->>'text' AS body FROM viewer_events WHERE scope_id=$1 AND source=$2 AND event_id=$3 AND event_type='chat'").bind(scope).bind(&e.source).bind(&e.event_id).fetch_optional(&mut *tx).await.map_err(database_error)?.ok_or_else(invalid)?;
                let author = r
                    .get::<Option<Uuid>, _>("viewer_id")
                    .ok_or_else(invalid)?
                    .to_string();
                if author != input.source.id && author != input.target.id
                    || !r.get::<String, _>("body").contains(&e.quote)
                {
                    return Err(invalid());
                }
                authors.insert(author);
            }
            if !input.admin_confirmed && !authors.contains(&input.source.id) {
                return Err(invalid());
            }
            let confirmation = if input.admin_confirmed {
                RelationConfirmation::Confirmed
            } else {
                RelationConfirmation::Claimed
            };
            let f = RelationshipFact {
                id: Uuid::new_v4().to_string(),
                version: 1,
                source: input.source.clone(),
                target: input.target.clone(),
                kind: input.kind,
                confirmation,
                evidence: input.evidence.clone(),
                expires_at_ms: input.expires_at_ms,
                deleted: false,
            };
            let v = encode(&f);
            sqlx::query("INSERT INTO relationship_facts(scope_id,id,source_kind,source_id,target_kind,target_id,kind,confirmation,evidence,expires_at_ms,created_at_ms,updated_at_ms) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$11)").bind(scope).bind(id(&f.id)?).bind(entity_kind(f.source.kind)).bind(&f.source.id).bind(entity_kind(f.target.kind)).bind(&f.target.id).bind(relation_kind(f.kind)).bind(if confirmation==RelationConfirmation::Confirmed{"confirmed"}else{"claimed"}).bind(sqlx::types::Json(v["evidence"].clone())).bind(f.expires_at_ms.map(timestamp).transpose()?).bind(timestamp(now)?).execute(&mut *tx).await.map_err(database_error)?;
            enqueue(&mut tx, scope, &f, timestamp(now)?).await?;
            audit(
                &mut tx,
                scope,
                &input.request_key,
                &fingerprint,
                Some(id(&f.id)?),
                &input.reason,
                timestamp(now)?,
                v,
            )
            .await?;
            bump(&mut tx, scope).await?;
            tx.commit().await.map_err(database_error)?;
            Ok(f)
        })
    }
    fn change<'a>(
        &'a self,
        scope: &'a str,
        fact: &'a str,
        expected: u64,
        action: RelationAction,
        reason: &'a str,
        key: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, RelationshipFact> {
        Box::pin(async move {
            admin(scope, key, reason, now)?;
            let mut tx = self.begin().await?;
            lock(&mut tx, scope).await?;
            let fingerprint = format!("change:{fact}:{expected}:{action:?}:{reason:?}");
            if let Some(old) = previous(&mut tx, scope, key, &fingerprint).await? {
                return decode(&old);
            }
            let mut f = fetch(&mut tx, scope, id(fact)?).await?;
            if f.version != expected || f.deleted {
                return Err(invalid());
            }
            f.version = f.version.checked_add(1).ok_or_else(invalid)?;
            match action {
                RelationAction::Confirm => f.confirmation = RelationConfirmation::Confirmed,
                RelationAction::Revoke => f.confirmation = RelationConfirmation::Claimed,
                RelationAction::Delete => f.deleted = true,
            };
            sqlx::query("UPDATE relationship_facts SET version=$3,confirmation=$4,deleted=$5,updated_at_ms=$6 WHERE scope_id=$1 AND id=$2").bind(scope).bind(id(fact)?).bind(timestamp(f.version)?).bind(if f.confirmation==RelationConfirmation::Confirmed{"confirmed"}else{"claimed"}).bind(f.deleted).bind(timestamp(now)?).execute(&mut *tx).await.map_err(database_error)?;
            enqueue(&mut tx, scope, &f, timestamp(now)?).await?;
            audit(
                &mut tx,
                scope,
                key,
                &fingerprint,
                Some(id(fact)?),
                reason,
                timestamp(now)?,
                encode(&f),
            )
            .await?;
            bump(&mut tx, scope).await?;
            tx.commit().await.map_err(database_error)?;
            Ok(f)
        })
    }
    fn list_admin<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        limit: u32,
        offset: u64,
    ) -> ViewerStoreFuture<'a, Vec<RelationshipFact>> {
        Box::pin(async move {
            validate_scope(scope)?;
            id(viewer)?;
            let (limit, offset) = page(limit, offset)?;
            let mut tx = self.begin().await?;
            let rows=sqlx::query_scalar::<_,sqlx::types::Json<Value>>("SELECT to_jsonb(f) FROM relationship_facts f WHERE scope_id=$1 AND ((source_kind='viewer' AND source_id=$2) OR (target_kind='viewer' AND target_id=$2)) ORDER BY updated_at_ms DESC,id LIMIT $3 OFFSET $4").bind(scope).bind(viewer).bind(limit).bind(offset).fetch_all(&mut *tx).await.map_err(database_error)?;
            let result = rows
                .into_iter()
                .map(|r| decode(&r.0))
                .collect::<Result<Vec<_>, _>>()?;
            tx.commit().await.map_err(database_error)?;
            Ok(result)
        })
    }
    fn query<'a>(
        &'a self,
        scope: &'a str,
        viewer: &'a str,
        depth: u8,
        limit: u32,
        now: u64,
    ) -> ViewerStoreFuture<'a, Vec<RelationshipFact>> {
        Box::pin(async move {
            validate_scope(scope)?;
            id(viewer)?;
            let (limit, _) = page(limit, 0)?;
            if !(1..=2).contains(&depth) {
                return Err(invalid());
            }
            let mut tx = self.begin().await?;
            let mut frontier = vec![("viewer".to_owned(), viewer.to_owned())];
            let mut facts = std::collections::BTreeMap::new();
            let mut seen = std::collections::BTreeSet::new();
            for _ in 0..depth {
                let mut next = Vec::new();
                for (kind, entity) in frontier {
                    if !seen.insert((kind.clone(), entity.clone())) {
                        continue;
                    }
                    let rows=sqlx::query_scalar::<_,sqlx::types::Json<Value>>("SELECT to_jsonb(f) FROM relationship_facts f WHERE scope_id=$1 AND NOT deleted AND confirmation='confirmed' AND (expires_at_ms IS NULL OR expires_at_ms>$4) AND NOT EXISTS (SELECT 1 FROM memory_evidence me JOIN memories m ON m.scope_id=me.scope_id AND m.id=me.memory_id CROSS JOIN LATERAL jsonb_array_elements(f.evidence) proof(item) WHERE me.scope_id=f.scope_id AND me.source=proof.item->>'source' AND me.event_id=proof.item->>'event_id' AND (m.deleted OR m.status='expired' OR m.expires_at_ms<=$4)) AND ((source_kind=$2 AND source_id=$3) OR (target_kind=$2 AND target_id=$3)) ORDER BY updated_at_ms DESC,id LIMIT $5").bind(scope).bind(&kind).bind(&entity).bind(timestamp(now)?).bind(limit).fetch_all(&mut *tx).await.map_err(database_error)?;
                    for r in rows {
                        let f = effective_expiry(&mut tx, scope, decode(&r.0)?).await?;
                        next.push((entity_kind(f.source.kind).into(), f.source.id.clone()));
                        next.push((entity_kind(f.target.kind).into(), f.target.id.clone()));
                        facts.insert(f.id.clone(), f);
                        if facts.len() >= limit as usize {
                            break;
                        }
                    }
                    if facts.len() >= limit as usize {
                        break;
                    }
                }
                if facts.len() >= limit as usize {
                    break;
                }
                frontier = next;
            }
            tx.commit().await.map_err(database_error)?;
            Ok(facts.into_values().collect())
        })
    }
    fn validate_graph<'a>(
        &'a self,
        scope: &'a str,
        refs: &'a [GraphReference],
        now: u64,
    ) -> ViewerStoreFuture<'a, Vec<RelationshipFact>> {
        Box::pin(async move {
            validate_scope(scope)?;
            if refs.len() > 100 {
                return Err(invalid());
            }
            let mut tx = self.begin().await?;
            let mut results = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for r in refs {
                if !seen.insert(&r.fact_id) {
                    continue;
                }
                let row=sqlx::query_scalar::<_,sqlx::types::Json<Value>>("SELECT to_jsonb(f) FROM relationship_facts f WHERE scope_id=$1 AND id=$2 AND version=$3 AND NOT deleted AND confirmation='confirmed' AND (expires_at_ms IS NULL OR expires_at_ms>$4) AND NOT EXISTS (SELECT 1 FROM memory_evidence me JOIN memories m ON m.scope_id=me.scope_id AND m.id=me.memory_id CROSS JOIN LATERAL jsonb_array_elements(f.evidence) proof(item) WHERE me.scope_id=f.scope_id AND me.source=proof.item->>'source' AND me.event_id=proof.item->>'event_id' AND (m.deleted OR m.status='expired' OR m.expires_at_ms<=$4))").bind(scope).bind(id(&r.fact_id)?).bind(timestamp(r.version)?).bind(timestamp(now)?).fetch_optional(&mut *tx).await.map_err(database_error)?;
                if let Some(v) = row {
                    results.push(effective_expiry(&mut tx, scope, decode(&v.0)?).await?);
                }
            }
            tx.commit().await.map_err(database_error)?;
            Ok(results)
        })
    }
    fn claim_outbox<'a>(
        &'a self,
        scope: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, Option<GraphOutboxLease>> {
        Box::pin(queue::claim(self, scope, now))
    }
    fn finish_outbox<'a>(
        &'a self,
        scope: &'a str,
        fact: &'a str,
        token: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(queue::finish(self, scope, fact, token, now, false))
    }
    fn fail_outbox<'a>(
        &'a self,
        scope: &'a str,
        fact: &'a str,
        token: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(queue::finish(self, scope, fact, token, now, true))
    }
    fn status<'a>(&'a self, scope: &'a str, now: u64) -> ViewerStoreFuture<'a, GraphSyncStatus> {
        Box::pin(queue::status(self, scope, now))
    }
    fn rebuild<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
        reason: &'a str,
        now: u64,
    ) -> ViewerStoreFuture<'a, u64> {
        Box::pin(queue::rebuild(self, scope, key, reason, now))
    }
}
