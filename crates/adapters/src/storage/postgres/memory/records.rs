use super::*;
async fn evidence(
    tx: &mut Transaction<'_, Postgres>,
    s: &str,
    m: Uuid,
) -> Result<Vec<Evidence>, ViewerStoreError> {
    let rows=sqlx::query("SELECT * FROM memory_evidence WHERE scope_id=$1 AND memory_id=$2 ORDER BY occurred_at_ms,source,event_id LIMIT 32").bind(s).bind(m).fetch_all(&mut **tx).await.map_err(database_error)?;
    Ok(rows
        .into_iter()
        .map(|r| Evidence {
            source: r.get("source"),
            event_id: r.get("event_id"),
            quote: r.get("quote"),
            occurred_at_ms: r.get("occurred_at_ms"),
            day: r.get("day"),
        })
        .collect())
}
pub(super) async fn list(
    tx: &mut Transaction<'_, Postgres>,
    s: &str,
    v: Uuid,
    l: u32,
    n: i64,
    active: bool,
    query: Option<&QueryEmbedding>,
) -> Result<Vec<MemoryRecord>, ViewerStoreError> {
    let (model, dim, vector) = if let Some(q) = query {
        (
            q.model.clone(),
            q.vector.len() as i32,
            Some(vectors::literal(&q.vector)?),
        )
    } else {
        (String::new(), 0, None)
    };
    let rows=sqlx::query("SELECT m.* FROM memories m LEFT JOIN memory_vectors vec ON vec.scope_id=m.scope_id AND vec.memory_id=m.id AND vec.version=m.version AND vec.model=$6 AND vec.dimensions=$7 WHERE m.scope_id=$1 AND m.viewer_id=$2 AND (NOT $5 OR (NOT m.deleted AND m.status IN ('short_term','long_term') AND (m.expires_at_ms IS NULL OR m.expires_at_ms>$3))) ORDER BY (CASE WHEN m.status='long_term' THEN 1.0 WHEN m.status='short_term' THEN 0.8*POWER(0.5,GREATEST(0,$3-COALESCE((SELECT MIN(e.occurred_at_ms) FROM memory_evidence e WHERE e.scope_id=m.scope_id AND e.memory_id=m.id),m.created_at_ms))::double precision/604800000.0) ELSE 0.0 END)*(CASE WHEN $8::text IS NOT NULL AND vec.embedding IS NOT NULL THEN 1.0/(1.0+(vec.embedding <-> $8::vector)) ELSE 1.0 END) DESC,m.updated_at_ms DESC,m.id LIMIT $4").bind(s).bind(v).bind(n).bind(i64::from(l)).bind(active).bind(model).bind(dim).bind(vector).fetch_all(&mut **tx).await.map_err(database_error)?;
    let mut result = Vec::new();
    for row in rows {
        let m: Uuid = row.get("id");
        let expires: Option<i64> = row.get("expires_at_ms");
        let status = if expires.is_some_and(|v| v <= n) {
            MemoryStatus::Expired
        } else {
            match row.get::<String, _>("status").as_str() {
                "candidate" => MemoryStatus::Candidate,
                "short_term" => MemoryStatus::ShortTerm,
                "long_term" => MemoryStatus::LongTerm,
                _ => MemoryStatus::Expired,
            }
        };
        result.push(MemoryRecord {
            id: m.to_string(),
            viewer_id: v.to_string(),
            candidate: MemoryCandidate {
                key: row.get("key"),
                value: row.get("value"),
                kind: parse_kind(&row.get::<String, _>("kind"))?,
                evidence: evidence(tx, s, m).await?,
                explicit: false,
                confidence: 1.0,
                valid_until_ms: expires,
            },
            status,
            version: row.get("version"),
            locked: row.get("locked"),
            deleted: row.get("deleted"),
            expires_at_ms: expires,
        });
    }
    Ok(result)
}
pub(super) async fn context(
    store: &PostgresViewerEventStore,
    s: &str,
    viewers: &[String],
    q: Option<&QueryEmbedding>,
    n: i64,
) -> Result<MemorySnapshot, ViewerStoreError> {
    if viewers.len() > 32 {
        return Err(invalid());
    }
    let mut tx = store.begin().await?;
    let revision = lock(&mut tx, s).await?;
    let mut unique = std::collections::BTreeSet::new();
    let mut groups = Vec::new();
    for viewer in viewers {
        let v = id(viewer)?;
        if unique.insert(v) {
            groups.push(list(&mut tx, s, v, 8, n, true, q).await?.into_iter());
        }
    }
    let mut records = Vec::new();
    let mut chars = 0;
    for _ in 0..8 {
        for group in &mut groups {
            if records.len() == 8 {
                break;
            }
            if let Some(mut record) = group.next() {
                let size =
                    record.candidate.value.chars().count() + record.candidate.key.chars().count();
                if chars + size > 4800 {
                    continue;
                }
                chars += size;
                record.candidate.evidence.clear();
                records.push(record)
            }
        }
    }
    tx.commit().await.map_err(database_error)?;
    Ok(MemorySnapshot { revision, records })
}
pub(super) async fn merge(
    tx: &mut Transaction<'_, Postgres>,
    s: &str,
    source: &MemorySource,
    c: &MemoryCandidate,
    n: i64,
) -> Result<(), ViewerStoreError> {
    let viewer = id(&source.viewer_id)?;
    let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_suppressions WHERE scope_id=$1 AND viewer_id=$2 AND key=$3 AND ((source='' AND event_id='') OR (source=$4 AND event_id=$5))) OR EXISTS(SELECT 1 FROM memories WHERE scope_id=$1 AND viewer_id=$2 AND key=$3 AND locked)").bind(s).bind(viewer).bind(&c.key).bind(&source.source).bind(&source.event_id).fetch_one(&mut **tx).await.map_err(database_error)?;
    if blocked {
        return Ok(());
    }
    let existing=sqlx::query("SELECT id,expires_at_ms FROM memories WHERE scope_id=$1 AND viewer_id=$2 AND key=$3 AND value=$4 AND NOT deleted ORDER BY updated_at_ms DESC LIMIT 1 FOR UPDATE").bind(s).bind(viewer).bind(&c.key).bind(&c.value).fetch_optional(&mut **tx).await.map_err(database_error)?;
    let m = existing
        .as_ref()
        .map(|r| r.get::<Uuid, _>("id"))
        .unwrap_or_else(Uuid::new_v4);
    let mut combined = c.clone();
    let initial = assess(c, std::slice::from_ref(source), n).map_err(|_| invalid())?;
    if existing.is_some() {
        let original = evidence(tx, s, m).await?;
        let mut prior = Vec::new();
        for e in original {
            let suppressed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_suppressions WHERE scope_id=$1 AND viewer_id=$2 AND key=$3 AND source=$4 AND event_id=$5)").bind(s).bind(viewer).bind(&c.key).bind(&e.source).bind(&e.event_id).fetch_one(&mut **tx).await.map_err(database_error)?;
            if !suppressed && e.quote.contains(&c.value) {
                prior.push(e)
            }
        }
        if c.evidence.iter().all(|e| {
            prior
                .iter()
                .any(|p| p.source == e.source && p.event_id == e.event_id)
        }) {
            return Ok(());
        }
        for e in &c.evidence {
            if !prior
                .iter()
                .any(|p| p.source == e.source && p.event_id == e.event_id)
            {
                prior.push(e.clone())
            }
        }
        prior.truncate(32);
        combined.evidence = prior;
    }
    let mut sources = Vec::new();
    for e in &combined.evidence {
        let text:Option<String>=sqlx::query_scalar("SELECT payload->>'text' FROM viewer_events WHERE scope_id=$1 AND viewer_id=$2 AND source=$3 AND event_id=$4") .bind(s).bind(viewer).bind(&e.source).bind(&e.event_id).fetch_optional(&mut **tx).await.map_err(database_error)?.flatten();
        let Some(text) = text.filter(|t| !t.is_empty()) else {
            return Ok(());
        };
        sources.push(MemorySource {
            source: e.source.clone(),
            viewer_id: source.viewer_id.clone(),
            event_id: e.event_id.clone(),
            text,
            occurred_at_ms: e.occurred_at_ms,
            day: e.day,
        });
    }
    let mut assessment = assess(&combined, &sources, n).map_err(|_| invalid())?;
    if assessment.status != MemoryStatus::LongTerm {
        if let Some(old) = existing
            .as_ref()
            .and_then(|r| r.get::<Option<i64>, _>("expires_at_ms"))
        {
            assessment.expires_at_ms = Some(assessment.expires_at_ms.map_or(old, |v| v.min(old)));
        }
    }
    // Only a directly confirmed latest self-report supersedes an older preference.
    let direct_confirmation = initial.status == MemoryStatus::LongTerm;
    if matches!(c.kind, MemoryKind::Preference | MemoryKind::PreferredName)
        && assessment.status == MemoryStatus::LongTerm
    {
        let newer:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memories m JOIN memory_evidence e ON e.scope_id=m.scope_id AND e.memory_id=m.id WHERE m.scope_id=$1 AND m.viewer_id=$2 AND m.key=$3 AND m.id<>$4 AND NOT m.deleted AND e.occurred_at_ms>$5)").bind(s).bind(viewer).bind(&c.key).bind(m).bind(source.occurred_at_ms).fetch_one(&mut **tx).await.map_err(database_error)?;
        if newer {
            assessment.status = MemoryStatus::Candidate;
            assessment.expires_at_ms = Some(source.occurred_at_ms.saturating_add(7 * DAY_MS));
        }
    }

    if assessment.status == MemoryStatus::LongTerm
        && direct_confirmation
        && matches!(c.kind, MemoryKind::Preference | MemoryKind::PreferredName)
    {
        sqlx::query("UPDATE memories SET deleted=true,version=version+1,updated_at_ms=$4 WHERE scope_id=$1 AND viewer_id=$2 AND key=$3 AND id<>$5 AND NOT locked AND NOT deleted AND NOT EXISTS(SELECT 1 FROM memory_evidence e WHERE e.scope_id=memories.scope_id AND e.memory_id=memories.id AND e.occurred_at_ms>$6)").bind(s).bind(viewer).bind(&c.key).bind(n).bind(m).bind(source.occurred_at_ms).execute(&mut **tx).await.map_err(database_error)?;
        sqlx::query("DELETE FROM memory_vectors v USING memories m WHERE v.scope_id=$1 AND m.scope_id=v.scope_id AND m.id=v.memory_id AND m.deleted").bind(s).execute(&mut **tx).await.map_err(database_error)?;
    }
    sqlx::query("INSERT INTO memories(id,scope_id,viewer_id,key,value,kind,status,created_at_ms,updated_at_ms,expires_at_ms) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$8,$9) ON CONFLICT(id) DO UPDATE SET status=EXCLUDED.status,expires_at_ms=EXCLUDED.expires_at_ms,updated_at_ms=EXCLUDED.updated_at_ms,version=memories.version+1").bind(m).bind(s).bind(viewer).bind(&c.key).bind(&c.value).bind(kind(c.kind)).bind(status(assessment.status)).bind(n).bind(assessment.expires_at_ms).execute(&mut **tx).await.map_err(database_error)?;
    for e in &c.evidence {
        sqlx::query("INSERT INTO memory_evidence(scope_id,memory_id,source,event_id,quote,occurred_at_ms,day) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING").bind(s).bind(m).bind(&e.source).bind(&e.event_id).bind(&e.quote).bind(e.occurred_at_ms).bind(e.day).execute(&mut **tx).await.map_err(database_error)?;
    }
    super::super::relationships::derived::from_memory(tx, s, m, n).await?;
    for (state, cap) in [("candidate", 50_i64), ("short_term", 200)] {
        sqlx::query("UPDATE memories SET deleted=true,version=version+1 WHERE scope_id=$1 AND id IN(SELECT id FROM memories WHERE scope_id=$1 AND viewer_id=$2 AND status=$3 AND NOT deleted AND NOT locked ORDER BY updated_at_ms DESC,id OFFSET $4)").bind(s).bind(viewer).bind(state).bind(cap).execute(&mut **tx).await.map_err(database_error)?;
    }
    sqlx::query("DELETE FROM memory_vectors v USING memories m WHERE v.scope_id=$1 AND m.scope_id=v.scope_id AND m.id=v.memory_id AND (m.deleted OR v.version<>m.version)").bind(s).execute(&mut **tx).await.map_err(database_error)?;
    sqlx::query("INSERT INTO memory_tombstones(scope_id,memory_id,version,created_at_ms) SELECT scope_id,id,version,$2 FROM memories WHERE scope_id=$1 AND deleted ON CONFLICT(scope_id,memory_id) DO UPDATE SET version=EXCLUDED.version,created_at_ms=EXCLUDED.created_at_ms WHERE memory_tombstones.version<EXCLUDED.version").bind(s).bind(n).execute(&mut **tx).await.map_err(database_error)?;
    bump(tx, s).await
}
pub(super) async fn admin(
    store: &PostgresViewerEventStore,
    s: &str,
    r: &MemoryAdminRequest,
    operation: &str,
    value: Option<&str>,
    n: i64,
) -> Result<(), ViewerStoreError> {
    if r.reason.trim().is_empty()
        || r.reason.len() > 2048
        || r.actor.trim().is_empty()
        || r.actor.len() > 128
        || r.request_key.is_empty()
        || r.request_key.len() > 128
        || value.is_some_and(|v| v.trim().is_empty() || v.len() > 512)
    {
        return Err(invalid());
    }
    let viewer = id(&r.viewer_id)?;
    let memory = id(&r.memory_id)?;
    let mut tx = store.begin().await?;
    lock(&mut tx, s).await?;
    if let Some(old) =
        sqlx::query("SELECT * FROM memory_audit WHERE scope_id=$1 AND request_key=$2")
            .bind(s)
            .bind(&r.request_key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(database_error)?
    {
        if old.get::<Uuid, _>("memory_id") != memory
            || old.get::<Uuid, _>("viewer_id") != viewer
            || old.get::<String, _>("operation") != operation
            || old.get::<Option<String>, _>("value").as_deref() != value
            || old.get::<String, _>("reason") != r.reason
            || old.get::<String, _>("actor") != r.actor
            || old.get::<i64, _>("expected_version") != r.expected_version
        {
            return Err(invalid());
        }
        return Ok(());
    }
    let row=sqlx::query("SELECT key FROM memories WHERE scope_id=$1 AND viewer_id=$2 AND id=$3 AND version=$4 AND NOT deleted FOR UPDATE").bind(s).bind(viewer).bind(memory).bind(r.expected_version).fetch_optional(&mut *tx).await.map_err(database_error)?.ok_or_else(invalid)?;
    sqlx::query("INSERT INTO memory_audit(scope_id,request_key,memory_id,viewer_id,operation,value,reason,actor,expected_version,created_at_ms) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)").bind(s).bind(&r.request_key).bind(memory).bind(viewer).bind(operation).bind(value).bind(&r.reason).bind(&r.actor).bind(r.expected_version).bind(n).execute(&mut *tx).await.map_err(database_error)?;
    sqlx::query("UPDATE memories SET value=COALESCE($4,value),deleted=($5='delete'),locked=($5<>'unfreeze'),status=CASE WHEN $5='correct' THEN 'long_term' ELSE status END,expires_at_ms=CASE WHEN $5='correct' THEN NULL ELSE expires_at_ms END,version=version+1,updated_at_ms=$3 WHERE scope_id=$1 AND id=$2").bind(s).bind(memory).bind(n).bind(value).bind(operation).execute(&mut *tx).await.map_err(database_error)?;
    sqlx::query("DELETE FROM memory_vectors WHERE scope_id=$1 AND memory_id=$2")
        .bind(s)
        .bind(memory)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    sqlx::query("DELETE FROM memory_embedding_jobs WHERE scope_id=$1 AND memory_id=$2")
        .bind(s)
        .bind(memory)
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    if operation == "unfreeze" {
        sqlx::query("DELETE FROM memory_suppressions WHERE scope_id=$1 AND viewer_id=$2 AND key=$3 AND source='' AND event_id=''").bind(s).bind(viewer).bind(row.get::<String,_>("key")).execute(&mut *tx).await.map_err(database_error)?;
    }
    if operation == "delete" || operation == "correct" {
        sqlx::query("INSERT INTO memory_suppressions(scope_id,viewer_id,key) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(s).bind(viewer).bind(row.get::<String,_>("key")).execute(&mut *tx).await.map_err(database_error)?;
        sqlx::query("INSERT INTO memory_suppressions(scope_id,viewer_id,key,source,event_id) SELECT $1,$2,$3,source,event_id FROM memory_evidence WHERE scope_id=$1 AND memory_id=$4 ON CONFLICT DO NOTHING").bind(s).bind(viewer).bind(row.get::<String,_>("key")).bind(memory).execute(&mut *tx).await.map_err(database_error)?;
        sqlx::query("INSERT INTO memory_tombstones(scope_id,memory_id,version,created_at_ms) VALUES($1,$2,$3,$4) ON CONFLICT(scope_id,memory_id) DO UPDATE SET version=EXCLUDED.version,created_at_ms=EXCLUDED.created_at_ms").bind(s).bind(memory).bind(r.expected_version+1).bind(n).execute(&mut *tx).await.map_err(database_error)?;
    }
    bump(&mut tx, s).await?;
    tx.commit().await.map_err(database_error)
}
pub(super) async fn maintenance(
    store: &PostgresViewerEventStore,
    s: &str,
    n: i64,
) -> Result<(), ViewerStoreError> {
    let mut tx = store.begin().await?;
    lock(&mut tx, s).await?;
    let affected=sqlx::query("UPDATE memories SET status='expired',version=version+1 WHERE scope_id=$1 AND status<>'expired' AND expires_at_ms<=$2").bind(s).bind(n).execute(&mut *tx).await.map_err(database_error)?.rows_affected();
    sqlx::query("DELETE FROM memory_vectors v USING memories m WHERE v.scope_id=$1 AND m.scope_id=v.scope_id AND m.id=v.memory_id AND (m.deleted OR m.status='expired' OR v.version<>m.version)").bind(s).execute(&mut *tx).await.map_err(database_error)?;
    sqlx::query("UPDATE viewer_events SET payload=jsonb_set(payload,'{text}','\"\"'::jsonb) WHERE scope_id=$1 AND event_type='chat' AND occurred_at_ms<$2 AND payload->>'text'<>''").bind(s).bind(n.saturating_sub(30*DAY_MS)).execute(&mut *tx).await.map_err(database_error)?;
    sqlx::query("UPDATE memory_jobs SET body='',state=CASE WHEN state IN('pending','running') THEN 'failed' ELSE state END,token=NULL WHERE scope_id=$1 AND occurred_at_ms<$2").bind(s).bind(n.saturating_sub(30*DAY_MS)).execute(&mut *tx).await.map_err(database_error)?;
    sqlx::query("UPDATE companionship_reply_events e SET chat_body=NULL FROM companionship_replies r WHERE e.scope_id=$1 AND r.scope_id=e.scope_id AND r.speech_id=e.speech_id AND r.generated_at_ms<$2 AND e.chat_body IS NOT NULL").bind(s).bind(n.saturating_sub(30*DAY_MS)).execute(&mut *tx).await.map_err(database_error)?;
    if affected > 0 {
        bump(&mut tx, s).await?
    }
    tx.commit().await.map_err(database_error)
}
