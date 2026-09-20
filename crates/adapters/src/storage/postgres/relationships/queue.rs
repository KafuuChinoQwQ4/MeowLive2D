//! 有界次数、持久租约和当前版本合并的图投影队列。
use super::*;
pub(super) async fn claim(
    store: &PostgresViewerEventStore,
    scope: &str,
    now: u64,
) -> Result<Option<GraphOutboxLease>, ViewerStoreError> {
    let mut tx = store.begin().await?;
    lock(&mut tx, scope).await?;
    let now = timestamp(now)?;
    sqlx::query("UPDATE relationship_outbox SET state='failed',token=NULL,lease_until_ms=NULL WHERE scope_id=$1 AND state='leased' AND lease_until_ms<=$2 AND attempts>=3").bind(scope).bind(now).execute(&mut *tx).await.map_err(database_error)?;
    let candidate=sqlx::query_scalar::<_,Uuid>("SELECT fact_id FROM relationship_outbox WHERE scope_id=$1 AND attempts<3 AND ((state='pending' AND available_at_ms<=$2) OR (state='leased' AND lease_until_ms<=$2)) ORDER BY created_at_ms,fact_id LIMIT 1 FOR UPDATE SKIP LOCKED").bind(scope).bind(now).fetch_optional(&mut *tx).await.map_err(database_error)?;
    let Some(fact) = candidate else {
        tx.commit().await.map_err(database_error)?;
        return Ok(None);
    };
    let token = Uuid::new_v4();
    sqlx::query("UPDATE relationship_outbox SET state='leased',attempts=attempts+1,token=$3,lease_until_ms=$4 WHERE scope_id=$1 AND fact_id=$2").bind(scope).bind(fact).bind(token).bind(now.checked_add(30000).ok_or_else(invalid)?).execute(&mut *tx).await.map_err(database_error)?;
    let snapshot = fetch(&mut tx, scope, fact).await?;
    tx.commit().await.map_err(database_error)?;
    Ok(Some(GraphOutboxLease {
        id: fact.to_string(),
        token: token.to_string(),
        projection: GraphProjection {
            scope: scope.into(),
            fact: snapshot,
        },
    }))
}
pub(super) async fn finish(
    store: &PostgresViewerEventStore,
    scope: &str,
    fact: &str,
    token: &str,
    now: u64,
    failed: bool,
) -> Result<(), ViewerStoreError> {
    let mut tx = store.begin().await?;
    lock(&mut tx, scope).await?;
    let now = timestamp(now)?;
    let row=sqlx::query("SELECT attempts FROM relationship_outbox WHERE scope_id=$1 AND fact_id=$2 AND token=$3 AND state='leased' AND lease_until_ms>$4 FOR UPDATE").bind(scope).bind(id(fact)?).bind(id(token)?).bind(now).fetch_optional(&mut *tx).await.map_err(database_error)?.ok_or_else(invalid)?;
    let attempts = row.get::<i32, _>("attempts");
    let state = if !failed {
        "done"
    } else if attempts >= 3 {
        "failed"
    } else {
        "pending"
    };
    sqlx::query("UPDATE relationship_outbox SET state=$3,available_at_ms=$4,token=NULL,lease_until_ms=NULL WHERE scope_id=$1 AND fact_id=$2").bind(scope).bind(id(fact)?).bind(state).bind(now.checked_add(i64::from(attempts)*1000).ok_or_else(invalid)?).execute(&mut *tx).await.map_err(database_error)?;
    tx.commit().await.map_err(database_error)?;
    Ok(())
}
pub(super) async fn status(
    store: &PostgresViewerEventStore,
    scope: &str,
    now: u64,
) -> Result<GraphSyncStatus, ViewerStoreError> {
    validate_scope(scope)?;
    timestamp(now)?;
    let mut tx = store.begin().await?;
    let r=sqlx::query("SELECT count(*) FILTER(WHERE state='pending') AS pending,count(*) FILTER(WHERE state='leased') AS leased,count(*) FILTER(WHERE state='failed') AS failed,min(created_at_ms) FILTER(WHERE state IN ('pending','leased')) AS oldest FROM relationship_outbox WHERE scope_id=$1").bind(scope).fetch_one(&mut *tx).await.map_err(database_error)?;
    Ok(GraphSyncStatus {
        pending: r.get::<i64, _>("pending") as u64,
        leased: r.get::<i64, _>("leased") as u64,
        failed: r.get::<i64, _>("failed") as u64,
        oldest_pending_age_ms: r
            .get::<Option<i64>, _>("oldest")
            .map(|v| now.saturating_sub(v as u64)),
    })
}
pub(super) async fn rebuild(
    store: &PostgresViewerEventStore,
    scope: &str,
    key: &str,
    reason: &str,
    now: u64,
) -> Result<u64, ViewerStoreError> {
    admin(scope, key, reason, now)?;
    let mut tx = store.begin().await?;
    lock(&mut tx, scope).await?;
    let fingerprint = format!("rebuild:{reason:?}");
    if let Some(v) = previous(&mut tx, scope, key, &fingerprint).await? {
        return v.as_u64().ok_or_else(invalid);
    }
    let result=sqlx::query("INSERT INTO relationship_outbox(scope_id,fact_id,version,available_at_ms,created_at_ms) SELECT scope_id,id,version,$2,$2 FROM relationship_facts WHERE scope_id=$1 ON CONFLICT(scope_id,fact_id) DO UPDATE SET version=EXCLUDED.version,state='pending',attempts=0,available_at_ms=EXCLUDED.available_at_ms,created_at_ms=EXCLUDED.created_at_ms,token=NULL,lease_until_ms=NULL").bind(scope).bind(timestamp(now)?).execute(&mut *tx).await.map_err(database_error)?.rows_affected();
    audit(
        &mut tx,
        scope,
        key,
        &fingerprint,
        None,
        reason,
        timestamp(now)?,
        json!(result),
    )
    .await?;
    tx.commit().await.map_err(database_error)?;
    Ok(result)
}
