use super::*;
#[allow(clippy::too_many_arguments)]
pub(super) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    s: &str,
    a: Uuid,
    b: Uuid,
    p: &ViewerMergePreview,
    r: &ViewerMergeRequest,
    n: i64,
) -> Result<(), ViewerStoreError> {
    // Keep identity keys, original event IDs, audit IDs and original contribution origin.
    sqlx::query("UPDATE affinity_ledger SET original_viewer_id=COALESCE(original_viewer_id,viewer_id) WHERE scope_id=$1 AND viewer_id=ANY($2)").bind(s).bind(vec![a,b]).execute(&mut **tx).await.map_err(database_error)?;
    for table in [
        "viewer_identities",
        "viewer_events",
        "gift_ledger",
        "affinity_ledger",
        "companionship_reply_events",
        "companionship_completed_events",
    ] {
        sqlx::query(&format!(
            "UPDATE {table} SET viewer_id=$3 WHERE scope_id=$1 AND viewer_id=$2"
        ))
        .bind(s)
        .bind(a)
        .bind(b)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    }
    sqlx::query("INSERT INTO viewer_aliases(scope_id,viewer_id,alias,first_seen_at_ms,last_seen_at_ms) SELECT scope_id,$3,alias,first_seen_at_ms,last_seen_at_ms FROM viewer_aliases WHERE scope_id=$1 AND viewer_id=$2 ON CONFLICT(scope_id,viewer_id,alias) DO UPDATE SET first_seen_at_ms=LEAST(viewer_aliases.first_seen_at_ms,EXCLUDED.first_seen_at_ms),last_seen_at_ms=GREATEST(viewer_aliases.last_seen_at_ms,EXCLUDED.last_seen_at_ms)").bind(s).bind(a).bind(b).execute(&mut **tx).await.map_err(database_error)?;
    for (table, columns) in [
        ("viewer_presence", "day"),
        ("viewer_observed_sessions", "session_id"),
        ("companionship_daily_chats", "day,body_hash"),
        ("memory_suppressions", "key,source,event_id"),
    ] {
        sqlx::query(&format!("INSERT INTO {table}(scope_id,viewer_id,{columns}) SELECT scope_id,$3,{columns} FROM {table} WHERE scope_id=$1 AND viewer_id=$2 ON CONFLICT DO NOTHING")).bind(s).bind(a).bind(b).execute(&mut **tx).await.map_err(database_error)?;
        sqlx::query(&format!(
            "DELETE FROM {table} WHERE scope_id=$1 AND viewer_id=$2"
        ))
        .bind(s)
        .bind(a)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    }
    sqlx::query("DELETE FROM viewer_aliases WHERE scope_id=$1 AND viewer_id=$2")
        .bind(s)
        .bind(a)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    sqlx::query("INSERT INTO viewer_companionship(scope_id,viewer_id,last_seen_at_ms) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(s).bind(b).bind(n).execute(&mut **tx).await.map_err(database_error)?;
    sqlx::query("UPDATE viewer_companionship SET familiarity_milli=$3,affinity_milli=$4,last_seen_at_ms=GREATEST(last_seen_at_ms,COALESCE((SELECT last_seen_at_ms FROM viewer_companionship WHERE scope_id=$1 AND viewer_id=$5),0)) WHERE scope_id=$1 AND viewer_id=$2").bind(s).bind(b).bind(p.resulting_familiarity_milli).bind(p.resulting_affinity_milli).bind(a).execute(&mut **tx).await.map_err(database_error)?;
    sqlx::query("UPDATE viewer_companionship SET familiarity_milli=0,affinity_milli=0 WHERE scope_id=$1 AND viewer_id=$2").bind(s).bind(a).execute(&mut **tx).await.map_err(database_error)?;
    for (kind, delta) in [
        (
            "merge_familiarity",
            p.resulting_familiarity_milli - p.source.familiarity_milli - p.target.familiarity_milli,
        ),
        (
            "merge_affinity",
            p.resulting_affinity_milli - p.source.affinity_milli - p.target.affinity_milli,
        ),
    ] {
        sqlx::query("INSERT INTO affinity_ledger(id,scope_id,viewer_id,kind,day,computed_delta_milli,applied_delta_milli,reason,actor,created_at_ms,request_key,fingerprint,original_viewer_id) VALUES($1,$2,$3,$4,$5,$6,$6,$7,$8,$9,$10,$11,$3)").bind(Uuid::new_v4()).bind(s).bind(b).bind(kind).bind(n.div_euclid(86400000)).bind(delta).bind(&r.reason).bind(&r.actor).bind(n).bind(format!("merge:{kind}:{}",r.request_key)).bind(&r.fingerprint).execute(&mut **tx).await.map_err(database_error)?;
    }
    // Every duplicate key is frozen, including historical/deleted rows: never resurrect a correction.
    sqlx::query("UPDATE memories SET locked=true,status=CASE WHEN deleted THEN status ELSE 'candidate' END,expires_at_ms=CASE WHEN deleted THEN expires_at_ms ELSE LEAST(COALESCE(expires_at_ms,$4+604800000),$4+604800000) END,version=version+1,updated_at_ms=$4 WHERE scope_id=$1 AND viewer_id=ANY($2) AND key IN(SELECT key FROM memories WHERE scope_id=$1 AND viewer_id=ANY($2) GROUP BY key HAVING COUNT(DISTINCT viewer_id)>1) AND $3::uuid IS NOT NULL").bind(s).bind(vec![a,b]).bind(b).bind(n).execute(&mut **tx).await.map_err(database_error)?;
    sqlx::query("DELETE FROM memory_vectors v USING memories m WHERE v.scope_id=$1 AND m.scope_id=v.scope_id AND m.id=v.memory_id AND m.viewer_id=ANY($2)").bind(s).bind(vec![a,b]).execute(&mut **tx).await.map_err(database_error)?;
    sqlx::query("DELETE FROM memory_embedding_jobs j USING memories m WHERE j.scope_id=$1 AND m.scope_id=j.scope_id AND m.id=j.memory_id AND m.viewer_id=ANY($2)").bind(s).bind(vec![a,b]).execute(&mut **tx).await.map_err(database_error)?;
    sqlx::query("UPDATE memories SET viewer_id=$3,version=version+1,updated_at_ms=$4 WHERE scope_id=$1 AND viewer_id=$2").bind(s).bind(a).bind(b).bind(n).execute(&mut **tx).await.map_err(database_error)?;
    sqlx::query("UPDATE memory_jobs SET viewer_id=CASE WHEN viewer_id=$2 THEN $3 ELSE viewer_id END,state=CASE WHEN state='running' THEN CASE WHEN attempts>=3 THEN 'failed' ELSE 'pending' END ELSE state END,token=NULL,lease_until_ms=NULL,available_at_ms=$4 WHERE scope_id=$1 AND viewer_id=ANY($5)").bind(s).bind(a).bind(b).bind(n).bind(vec![a,b]).execute(&mut **tx).await.map_err(database_error)?;
    let rows=sqlx::query("UPDATE relationship_facts SET source_id=CASE WHEN source_kind='viewer' AND source_id=$2 THEN $3 ELSE source_id END,target_id=CASE WHEN target_kind='viewer' AND target_id=$2 THEN $3 ELSE target_id END,confirmation='claimed',version=version+1,updated_at_ms=$4 WHERE scope_id=$1 AND ((source_kind='viewer' AND source_id IN($2,$3)) OR(target_kind='viewer' AND target_id IN($2,$3))) RETURNING id,version").bind(s).bind(a.to_string()).bind(b.to_string()).bind(n).fetch_all(&mut **tx).await.map_err(database_error)?;
    for row in rows {
        let fact: Uuid = row.get("id");
        sqlx::query("UPDATE relationship_facts SET deleted=true WHERE scope_id=$1 AND id=$2 AND source_kind=target_kind AND source_id=target_id").bind(s).bind(fact).execute(&mut **tx).await.map_err(database_error)?;
        sqlx::query("INSERT INTO relationship_outbox(scope_id,fact_id,version,available_at_ms,created_at_ms) VALUES($1,$2,$3,$4,$4) ON CONFLICT(scope_id,fact_id) DO UPDATE SET version=EXCLUDED.version,state='pending',attempts=0,available_at_ms=EXCLUDED.available_at_ms,token=NULL,lease_until_ms=NULL").bind(s).bind(fact).bind(row.get::<i64,_>("version")).bind(n).execute(&mut **tx).await.map_err(database_error)?;
    }
    sqlx::query(
        "UPDATE viewers SET merged_into=$3 WHERE scope_id=$1 AND (id=$2 OR merged_into=$2)",
    )
    .bind(s)
    .bind(a)
    .bind(b)
    .execute(&mut **tx)
    .await
    .map_err(database_error)?;
    Ok(())
}
