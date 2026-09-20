//! 仅从已校验记忆派生最小关系事实；不绑定昵称，也不覆盖管理员关系状态。
use super::*;
use meowlive_domain::memory::{
    Evidence, MemoryCandidate, MemoryKind, MemorySource, MemoryStatus, assess,
};
pub(in crate::storage::postgres) async fn from_memory(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    memory_id: Uuid,
    now: i64,
) -> Result<(), ViewerStoreError> {
    let Some(row)=sqlx::query("SELECT m.viewer_id,m.value,m.kind,m.status,m.expires_at_ms FROM memories m JOIN viewers v ON v.scope_id=m.scope_id AND v.id=m.viewer_id WHERE m.scope_id=$1 AND m.id=$2 AND NOT m.deleted AND NOT m.locked AND v.merged_into IS NULL AND (m.expires_at_ms IS NULL OR m.expires_at_ms>$3)").bind(scope).bind(memory_id).bind(now).fetch_optional(&mut **tx).await.map_err(database_error)? else{return Ok(())};
    let value: String = row.get("value");
    if value.trim().is_empty() || value.chars().count() > 128 || value.chars().any(char::is_control)
    {
        return Ok(());
    }
    let memory_kind: String = row.get("kind");
    let status: String = row.get("status");
    let (kind, target_kind, confirmation, rule_kind) = match (memory_kind.as_str(), status.as_str())
    {
        ("preference", "long_term") => (
            "shared_interest",
            "topic",
            "confirmed",
            MemoryKind::Preference,
        ),
        ("experience", "short_term") => (
            "participated",
            "activity",
            "confirmed",
            MemoryKind::Experience,
        ),
        ("third_party_claim", "candidate") => (
            "mention",
            "unresolved",
            "claimed",
            MemoryKind::ThirdPartyClaim,
        ),
        _ => return Ok(()),
    };
    let viewer: Uuid = row.get("viewer_id");
    let expires: Option<i64> = row.get("expires_at_ms");
    let evidence_rows=sqlx::query("SELECT e.source,e.event_id,e.quote,e.occurred_at_ms,e.day,v.payload->>'text' AS body,v.occurred_at_ms AS event_time FROM memory_evidence e JOIN viewer_events v ON v.scope_id=e.scope_id AND v.source=e.source AND v.event_id=e.event_id AND v.viewer_id=$3 WHERE e.scope_id=$1 AND e.memory_id=$2 AND v.event_type='chat' ORDER BY e.occurred_at_ms,e.source,e.event_id LIMIT 8").bind(scope).bind(memory_id).bind(viewer).fetch_all(&mut **tx).await.map_err(database_error)?;
    let mut accepted = Vec::new();
    for e in evidence_rows {
        let quote: String = e.get("quote");
        let body: Option<String> = e.get("body");
        let Some(body) = body else { continue };
        if quote.chars().count() > 300
            || quote != body
            || e.get::<i64, _>("occurred_at_ms") != e.get::<i64, _>("event_time")
        {
            continue;
        }
        let source = MemorySource {
            source: e.get("source"),
            viewer_id: viewer.to_string(),
            event_id: e.get("event_id"),
            text: body,
            occurred_at_ms: e.get("event_time"),
            day: e.get("day"),
        };
        let supports = if rule_kind == MemoryKind::ThirdPartyClaim {
            quote == format!("我认识{value}") || quote == format!("我和{value}是朋友")
        } else {
            let c = MemoryCandidate {
                key: memory_kind.clone(),
                value: value.clone(),
                kind: rule_kind,
                evidence: vec![Evidence::from_source(&source, &quote)],
                explicit: false,
                confidence: 1.0,
                valid_until_ms: expires,
            };
            assess(&c, std::slice::from_ref(&source), now).is_ok_and(|a| match rule_kind {
                MemoryKind::Preference => a.status == MemoryStatus::LongTerm,
                MemoryKind::Experience => a.status == MemoryStatus::ShortTerm,
                _ => false,
            })
        };
        if supports {
            accepted.push(json!({"source":source.source,"event_id":source.event_id,"quote":quote}));
        }
    }
    if accepted.is_empty() {
        return Ok(());
    }
    let inserted=sqlx::query("INSERT INTO relationship_facts(scope_id,id,version,source_kind,source_id,target_kind,target_id,kind,confirmation,evidence,expires_at_ms,created_at_ms,updated_at_ms) VALUES($1,$2,1,'viewer',$3,$4,$5,$6,$7,$8,$9,$10,$10) ON CONFLICT(scope_id,id) DO NOTHING").bind(scope).bind(memory_id).bind(viewer.to_string()).bind(target_kind).bind(value).bind(kind).bind(confirmation).bind(sqlx::types::Json(json!(accepted))).bind(expires).bind(now).execute(&mut **tx).await.map_err(database_error)?.rows_affected();
    if inserted > 0 {
        sqlx::query("INSERT INTO relationship_outbox(scope_id,fact_id,version,available_at_ms,created_at_ms) VALUES($1,$2,1,$3,$3) ON CONFLICT(scope_id,fact_id) DO NOTHING").bind(scope).bind(memory_id).bind(now).execute(&mut **tx).await.map_err(database_error)?;
    }
    Ok(())
}
