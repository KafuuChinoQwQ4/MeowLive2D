use super::*;
pub(super) fn literal(v: &[f32]) -> Result<String, ViewerStoreError> {
    if v.is_empty() || v.len() > 4096 || v.iter().any(|x| !x.is_finite()) {
        return Err(invalid());
    }
    Ok(format!(
        "[{}]",
        v.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ))
}
pub(super) async fn claim(
    store: &PostgresViewerEventStore,
    s: &str,
    model: &str,
    d: usize,
    n: i64,
) -> Result<Option<EmbeddingJob>, ViewerStoreError> {
    if model.is_empty() || model.len() > 256 || !(1..=4096).contains(&d) {
        return Err(invalid());
    }
    let mut tx = store.begin().await?;
    lock(&mut tx, s).await?;
    let row=sqlx::query("SELECT m.id,m.version,m.value FROM memories m LEFT JOIN memory_vectors v ON v.scope_id=m.scope_id AND v.memory_id=m.id AND v.model=$2 AND v.dimensions=$3 AND v.version=m.version LEFT JOIN memory_embedding_jobs j ON j.scope_id=m.scope_id AND j.memory_id=m.id AND j.model=$2 AND j.dimensions=$3 WHERE m.scope_id=$1 AND NOT m.deleted AND m.status IN('short_term','long_term') AND (m.expires_at_ms IS NULL OR m.expires_at_ms>$4) AND v.memory_id IS NULL AND (j.memory_id IS NULL OR j.version<>m.version OR (j.lease_until_ms<=$4 AND j.attempts<3)) ORDER BY m.updated_at_ms,m.id LIMIT 1").bind(s).bind(model).bind(d as i32).bind(n).fetch_optional(&mut *tx).await.map_err(database_error)?;
    let Some(row) = row else {
        tx.commit().await.map_err(database_error)?;
        return Ok(None);
    };
    let memory: Uuid = row.get("id");
    let version: i64 = row.get("version");
    let token = Uuid::new_v4();
    sqlx::query("INSERT INTO memory_embedding_jobs(scope_id,memory_id,version,model,dimensions,token,lease_until_ms) VALUES($1,$2,$3,$4,$5,$6,$7+60000) ON CONFLICT(scope_id,memory_id,model,dimensions) DO UPDATE SET version=EXCLUDED.version,token=EXCLUDED.token,lease_until_ms=EXCLUDED.lease_until_ms,attempts=CASE WHEN memory_embedding_jobs.version<>EXCLUDED.version THEN 1 ELSE memory_embedding_jobs.attempts+1 END").bind(s).bind(memory).bind(version).bind(model).bind(d as i32).bind(token).bind(n).execute(&mut *tx).await.map_err(database_error)?;
    tx.commit().await.map_err(database_error)?;
    Ok(Some(EmbeddingJob {
        memory_id: memory.to_string(),
        version,
        token: token.to_string(),
        text: row.get("value"),
        model: model.into(),
        dimensions: d,
    }))
}
pub(super) async fn store(
    store: &PostgresViewerEventStore,
    s: &str,
    j: &EmbeddingJob,
    b: &EmbeddingBatch,
    n: i64,
) -> Result<(), ViewerStoreError> {
    if b.model != j.model
        || b.dimensions != j.dimensions
        || b.vectors.len() != 1
        || b.vectors[0].len() != j.dimensions
    {
        return Err(invalid());
    }
    let vector = literal(&b.vectors[0])?;
    let mut tx = store.begin().await?;
    lock(&mut tx, s).await?;
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_embedding_jobs j JOIN memories m ON m.scope_id=j.scope_id AND m.id=j.memory_id WHERE j.scope_id=$1 AND j.memory_id=$2 AND j.token=$3 AND j.version=$4 AND j.model=$5 AND j.dimensions=$6 AND j.lease_until_ms>$7 AND m.version=j.version AND NOT m.deleted AND m.status IN('short_term','long_term') AND (m.expires_at_ms IS NULL OR m.expires_at_ms>$7))").bind(s).bind(id(&j.memory_id)?).bind(id(&j.token)?).bind(j.version).bind(&j.model).bind(j.dimensions as i32).bind(n).fetch_one(&mut *tx).await.map_err(database_error)?;
    if !valid {
        return Err(invalid());
    }
    sqlx::query("INSERT INTO memory_vectors(scope_id,memory_id,version,model,dimensions,embedding) VALUES($1,$2,$3,$4,$5,$6::vector) ON CONFLICT(scope_id,memory_id,model,dimensions) DO UPDATE SET version=EXCLUDED.version,embedding=EXCLUDED.embedding").bind(s).bind(id(&j.memory_id)?).bind(j.version).bind(&j.model).bind(j.dimensions as i32).bind(vector).execute(&mut *tx).await.map_err(database_error)?;
    sqlx::query(
        "DELETE FROM memory_embedding_jobs WHERE scope_id=$1 AND memory_id=$2 AND token=$3",
    )
    .bind(s)
    .bind(id(&j.memory_id)?)
    .bind(id(&j.token)?)
    .execute(&mut *tx)
    .await
    .map_err(database_error)?;
    tx.commit().await.map_err(database_error)
}
