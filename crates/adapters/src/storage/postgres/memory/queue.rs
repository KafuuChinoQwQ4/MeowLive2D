use super::*;
pub(super) async fn claim(
    store: &PostgresViewerEventStore,
    s: &str,
    n: i64,
) -> Result<Option<ExtractionJob>, ViewerStoreError> {
    let mut tx = store.begin().await?;
    let revision = lock(&mut tx, s).await?;
    sqlx::query("UPDATE memory_jobs SET state='failed',token=NULL WHERE scope_id=$1 AND state='running' AND lease_until_ms<=$2 AND attempts>=3").bind(s).bind(n).execute(&mut *tx).await.map_err(database_error)?;
    let running:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_jobs WHERE scope_id=$1 AND state='running' AND lease_until_ms>$2)").bind(s).bind(n).fetch_one(&mut *tx).await.map_err(database_error)?;
    if running {
        tx.commit().await.map_err(database_error)?;
        return Ok(None);
    }
    let token = Uuid::new_v4();
    let row=sqlx::query("UPDATE memory_jobs SET state='running',attempts=attempts+1,token=$3,lease_until_ms=$2+60000,revision=$4 WHERE id=(SELECT id FROM memory_jobs WHERE scope_id=$1 AND attempts<3 AND ((state='pending' AND available_at_ms<=$2) OR (state='running' AND lease_until_ms<=$2)) ORDER BY available_at_ms,id LIMIT 1) RETURNING *").bind(s).bind(n).bind(token).bind(revision).fetch_optional(&mut *tx).await.map_err(database_error)?;
    let result = row.map(|r| ExtractionJob {
        id: r.get::<Uuid, _>("id").to_string(),
        token: token.to_string(),
        revision,
        attempts: r.get("attempts"),
        sources: vec![MemorySource {
            source: r.get("source"),
            viewer_id: r.get::<Uuid, _>("viewer_id").to_string(),
            event_id: r.get("event_id"),
            text: r.get("body"),
            occurred_at_ms: r.get("occurred_at_ms"),
            day: r.get("day"),
        }],
    });
    tx.commit().await.map_err(database_error)?;
    Ok(result)
}
async fn validate(
    tx: &mut Transaction<'_, Postgres>,
    s: &str,
    j: &ExtractionJob,
    n: i64,
) -> Result<MemorySource, ViewerStoreError> {
    let row=sqlx::query("SELECT * FROM memory_jobs WHERE scope_id=$1 AND id=$2 AND token=$3 AND state='running' AND lease_until_ms>$4 FOR UPDATE").bind(s).bind(id(&j.id)?).bind(id(&j.token)?).bind(n).fetch_optional(&mut **tx).await.map_err(database_error)?.ok_or_else(invalid)?;
    Ok(MemorySource {
        source: row.get("source"),
        viewer_id: row.get::<Uuid, _>("viewer_id").to_string(),
        event_id: row.get("event_id"),
        text: row.get("body"),
        occurred_at_ms: row.get("occurred_at_ms"),
        day: row.get("day"),
    })
}
pub(super) async fn fail(
    store: &PostgresViewerEventStore,
    s: &str,
    j: &ExtractionJob,
    n: i64,
) -> Result<(), ViewerStoreError> {
    let mut tx = store.begin().await?;
    lock(&mut tx, s).await?;
    validate(&mut tx, s, j, n).await?;
    sqlx::query("UPDATE memory_jobs SET state=CASE WHEN attempts>=3 THEN 'failed' ELSE 'pending' END,available_at_ms=$3+(attempts*attempts*1000),token=NULL WHERE scope_id=$1 AND id=$2").bind(s).bind(id(&j.id)?).bind(n).execute(&mut *tx).await.map_err(database_error)?;
    tx.commit().await.map_err(database_error)
}
pub(super) async fn finish(
    store: &PostgresViewerEventStore,
    s: &str,
    j: &ExtractionJob,
    candidates: &[MemoryCandidate],
    n: i64,
) -> Result<(), ViewerStoreError> {
    if candidates.len() > 32 {
        return Err(invalid());
    }
    let mut tx = store.begin().await?;
    let revision = lock(&mut tx, s).await?;
    let source = validate(&mut tx, s, j, n).await?;
    if j.sources != [source.clone()] {
        return Err(invalid());
    }
    let stored_revision: i64 =
        sqlx::query_scalar("SELECT revision FROM memory_jobs WHERE scope_id=$1 AND id=$2")
            .bind(s)
            .bind(id(&j.id)?)
            .fetch_one(&mut *tx)
            .await
            .map_err(database_error)?;
    if stored_revision == revision && j.revision == revision {
        for candidate in candidates {
            assess(candidate, std::slice::from_ref(&source), n).map_err(|_| invalid())?;
            records::merge(&mut tx, s, &source, candidate, n).await?;
        }
    }
    sqlx::query(
        "UPDATE memory_jobs SET state='done',token=NULL,body='' WHERE scope_id=$1 AND id=$2",
    )
    .bind(s)
    .bind(id(&j.id)?)
    .execute(&mut *tx)
    .await
    .map_err(database_error)?;
    tx.commit().await.map_err(database_error)
}
