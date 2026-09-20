//! 审计化的人工恢复，不改变记忆事实或放行已失效的租约。
use super::*;
pub(super) async fn run(
    store: &PostgresViewerEventStore,
    s: &str,
    r: &MemoryMaintenanceRequest,
    operation: &str,
    n: i64,
) -> Result<(), ViewerStoreError> {
    if r.request_key.is_empty()
        || r.request_key.len() > 128
        || r.reason.trim().is_empty()
        || r.reason.len() > 2048
        || r.actor.trim().is_empty()
        || r.actor.len() > 128
    {
        return Err(invalid());
    }
    let mut tx = store.begin().await?;
    lock(&mut tx, s).await?;
    if let Some(row) = sqlx::query(
        "SELECT operation,reason,actor FROM memory_operations WHERE scope_id=$1 AND request_key=$2",
    )
    .bind(s)
    .bind(&r.request_key)
    .fetch_optional(&mut *tx)
    .await
    .map_err(database_error)?
    {
        if row.get::<String, _>("operation") != operation
            || row.get::<String, _>("reason") != r.reason
            || row.get::<String, _>("actor") != r.actor
        {
            return Err(invalid());
        }
        return Ok(());
    }
    sqlx::query("INSERT INTO memory_operations(scope_id,request_key,operation,reason,actor,created_at_ms) VALUES($1,$2,$3,$4,$5,$6)").bind(s).bind(&r.request_key).bind(operation).bind(&r.reason).bind(&r.actor).bind(n).execute(&mut *tx).await.map_err(database_error)?;
    if operation == "retry" {
        sqlx::query("UPDATE memory_jobs SET state='pending',attempts=0,available_at_ms=$2,token=NULL,lease_until_ms=NULL WHERE scope_id=$1 AND state='failed' AND body<>''").bind(s).bind(n).execute(&mut *tx).await.map_err(database_error)?;
        sqlx::query("DELETE FROM memory_embedding_jobs WHERE scope_id=$1 AND attempts>=3 AND lease_until_ms<=$2").bind(s).bind(n).execute(&mut *tx).await.map_err(database_error)?;
    } else {
        sqlx::query("DELETE FROM memory_vectors WHERE scope_id=$1")
            .bind(s)
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        sqlx::query("DELETE FROM memory_embedding_jobs WHERE scope_id=$1")
            .bind(s)
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
    }
    tx.commit().await.map_err(database_error)
}
