//! PostgreSQL 权威记忆；范围行锁串行化资料变更与后台租约。
use super::*;
use meowlive_application::ports::{memory::EmbeddingBatch, memory_store::*};
use meowlive_domain::memory::{
    DAY_MS, Evidence, MemoryCandidate, MemoryKind, MemorySource, MemoryStatus, assess,
};
mod operations;
mod queue;
mod records;
mod vectors;
fn invalid() -> ViewerStoreError {
    ViewerStoreError::new("invalid or stale memory operation")
}
fn id(s: &str) -> Result<Uuid, ViewerStoreError> {
    Uuid::parse_str(s).map_err(|_| invalid())
}
async fn lock(tx: &mut Transaction<'_, Postgres>, scope: &str) -> Result<i64, ViewerStoreError> {
    validate_scope(scope)?;
    sqlx::query("INSERT INTO memory_scopes(scope_id) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(scope)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    sqlx::query_scalar("SELECT revision FROM memory_scopes WHERE scope_id=$1 FOR UPDATE")
        .bind(scope)
        .fetch_one(&mut **tx)
        .await
        .map_err(database_error)
}
async fn bump(tx: &mut Transaction<'_, Postgres>, scope: &str) -> Result<(), ViewerStoreError> {
    sqlx::query("UPDATE memory_scopes SET revision=revision+1 WHERE scope_id=$1")
        .bind(scope)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    Ok(())
}
fn kind(k: MemoryKind) -> &'static str {
    match k {
        MemoryKind::Preference => "preference",
        MemoryKind::PreferredName => "preferred_name",
        MemoryKind::StableFact => "stable_fact",
        MemoryKind::Experience => "experience",
        MemoryKind::TemporaryState => "temporary_state",
        MemoryKind::ThirdPartyClaim => "third_party_claim",
        MemoryKind::SensitiveInference => "sensitive_inference",
    }
}
fn parse_kind(k: &str) -> Result<MemoryKind, ViewerStoreError> {
    Ok(match k {
        "preference" => MemoryKind::Preference,
        "preferred_name" => MemoryKind::PreferredName,
        "stable_fact" => MemoryKind::StableFact,
        "experience" => MemoryKind::Experience,
        "temporary_state" => MemoryKind::TemporaryState,
        "third_party_claim" => MemoryKind::ThirdPartyClaim,
        "sensitive_inference" => MemoryKind::SensitiveInference,
        _ => return Err(invalid()),
    })
}
fn status(s: MemoryStatus) -> &'static str {
    match s {
        MemoryStatus::Candidate => "candidate",
        MemoryStatus::ShortTerm => "short_term",
        MemoryStatus::LongTerm => "long_term",
        MemoryStatus::Expired => "expired",
    }
}
pub(super) async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
    source: &str,
    event_id: &str,
    now: i64,
    offset: i32,
) -> Result<(), ViewerStoreError> {
    sqlx::query("INSERT INTO memory_jobs(id,scope_id,viewer_id,source,event_id,body,occurred_at_ms,day,available_at_ms) SELECT $1,scope_id,viewer_id,source,event_id,payload->>'text',occurred_at_ms,FLOOR((occurred_at_ms::numeric+$6::bigint*60000)/86400000)::bigint,$5 FROM viewer_events WHERE scope_id=$2 AND source=$3 AND event_id=$4 AND viewer_id IS NOT NULL AND event_type='chat' AND length(payload->>'text')>0 ON CONFLICT(scope_id,source,event_id) DO NOTHING").bind(Uuid::new_v4()).bind(scope).bind(source).bind(event_id).bind(now).bind(offset).execute(&mut **tx).await.map_err(database_error)?;
    Ok(())
}
impl MemoryStore for PostgresViewerEventStore {
    fn retry_failed<'a>(
        &'a self,
        s: &'a str,
        r: &'a MemoryMaintenanceRequest,
        n: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(operations::run(self, s, r, "retry", n))
    }
    fn rebuild_vectors<'a>(
        &'a self,
        s: &'a str,
        r: &'a MemoryMaintenanceRequest,
        n: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(operations::run(self, s, r, "rebuild", n))
    }

    fn resolve_viewers<'a>(
        &'a self,
        scope: &'a str,
        events: &'a [meowlive_domain::event::LiveEvent],
    ) -> ViewerStoreFuture<'a, Vec<String>> {
        Box::pin(async move {
            validate_scope(scope)?;
            if events.len() > 100 {
                return Err(invalid());
            }
            let mut result = Vec::new();
            let mut tx = self.begin().await?;
            for event in events {
                let viewer:Option<Uuid>=sqlx::query_scalar("SELECT viewer_id FROM viewer_events WHERE scope_id=$1 AND source=$2 AND event_id=$3").bind(scope).bind(&event.source).bind(&event.id).fetch_optional(&mut *tx).await.map_err(database_error)?.flatten();
                result.push(viewer.map(|v| v.to_string()).unwrap_or_default());
            }
            tx.commit().await.map_err(database_error)?;
            Ok(result)
        })
    }

    fn claim_job<'a>(&'a self, s: &'a str, n: i64) -> ViewerStoreFuture<'a, Option<ExtractionJob>> {
        Box::pin(queue::claim(self, s, n))
    }
    fn finish_job<'a>(
        &'a self,
        s: &'a str,
        j: &'a ExtractionJob,
        c: &'a [MemoryCandidate],
        n: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(queue::finish(self, s, j, c, n))
    }
    fn fail_job<'a>(
        &'a self,
        s: &'a str,
        j: &'a ExtractionJob,
        n: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(queue::fail(self, s, j, n))
    }
    fn list<'a>(
        &'a self,
        s: &'a str,
        v: &'a str,
        l: u32,
        n: i64,
    ) -> ViewerStoreFuture<'a, Vec<MemoryRecord>> {
        Box::pin(async move {
            if !(1..=250).contains(&l) {
                return Err(invalid());
            }
            let mut tx = self.begin().await?;
            validate_scope(s)?;
            let result = records::list(&mut tx, s, id(v)?, l, n, false, None).await?;
            tx.commit().await.map_err(database_error)?;
            Ok(result)
        })
    }
    fn context<'a>(
        &'a self,
        s: &'a str,
        v: &'a [String],
        q: Option<&'a QueryEmbedding>,
        n: i64,
    ) -> ViewerStoreFuture<'a, MemorySnapshot> {
        Box::pin(records::context(self, s, v, q, n))
    }
    fn revision<'a>(&'a self, s: &'a str) -> ViewerStoreFuture<'a, i64> {
        Box::pin(async move {
            validate_scope(s)?;
            sqlx::query_scalar("SELECT revision FROM memory_scopes WHERE scope_id=$1")
                .bind(s)
                .fetch_optional(&self.pool)
                .await
                .map(|v| v.unwrap_or(0))
                .map_err(database_error)
        })
    }
    fn admin_correct<'a>(
        &'a self,
        s: &'a str,
        r: &'a MemoryAdminRequest,
        v: &'a str,
        n: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(records::admin(self, s, r, "correct", Some(v), n))
    }
    fn admin_delete<'a>(
        &'a self,
        s: &'a str,
        r: &'a MemoryAdminRequest,
        n: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(records::admin(self, s, r, "delete", None, n))
    }
    fn admin_freeze<'a>(
        &'a self,
        s: &'a str,
        r: &'a MemoryAdminRequest,
        f: bool,
        n: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(records::admin(
            self,
            s,
            r,
            if f { "freeze" } else { "unfreeze" },
            None,
            n,
        ))
    }
    fn claim_embedding<'a>(
        &'a self,
        s: &'a str,
        m: &'a str,
        d: usize,
        n: i64,
    ) -> ViewerStoreFuture<'a, Option<EmbeddingJob>> {
        Box::pin(vectors::claim(self, s, m, d, n))
    }
    fn store_embedding<'a>(
        &'a self,
        s: &'a str,
        j: &'a EmbeddingJob,
        b: &'a EmbeddingBatch,
        n: i64,
    ) -> ViewerStoreFuture<'a, ()> {
        Box::pin(vectors::store(self, s, j, b, n))
    }
    fn status<'a>(&'a self, s: &'a str) -> ViewerStoreFuture<'a, MemoryQueueStatus> {
        Box::pin(async move {
            validate_scope(s)?;
            let row=sqlx::query("SELECT COUNT(*) FILTER(WHERE state='pending') AS pending,COUNT(*) FILTER(WHERE state='running') AS running,COUNT(*) FILTER(WHERE state='failed') AS failed FROM memory_jobs WHERE scope_id=$1").bind(s).fetch_one(&self.pool).await.map_err(database_error)?;
            Ok(MemoryQueueStatus {
                pending: row.get("pending"),
                running: row.get("running"),
                failed: row.get("failed"),
                embedding_failed:sqlx::query_scalar("SELECT COUNT(*) FROM memory_embedding_jobs WHERE scope_id=$1 AND attempts>=3 AND lease_until_ms<=(EXTRACT(EPOCH FROM clock_timestamp())*1000)::bigint").bind(s).fetch_one(&self.pool).await.map_err(database_error)?,
                embedding_pending: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM memory_embedding_jobs WHERE scope_id=$1 AND (attempts<3 OR lease_until_ms>(EXTRACT(EPOCH FROM clock_timestamp())*1000)::bigint)",
                )
                .bind(s)
                .fetch_one(&self.pool)
                .await
                .map_err(database_error)?,
            })
        })
    }
    fn maintenance<'a>(&'a self, s: &'a str, n: i64) -> ViewerStoreFuture<'a, ()> {
        Box::pin(records::maintenance(self, s, n))
    }
}
