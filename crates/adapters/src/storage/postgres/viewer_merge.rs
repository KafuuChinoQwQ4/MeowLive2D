//! 显式身份合并；源档案保留审计墓碑，权威事实迁往目标。
use super::*;
use meowlive_application::ports::viewer_merge::*;
use sha2::{Digest, Sha256};
mod migrate;
fn invalid() -> ViewerStoreError {
    ViewerStoreError::new("invalid or stale viewer merge")
}
fn id(s: &str) -> Result<Uuid, ViewerStoreError> {
    Uuid::parse_str(s).map_err(|_| invalid())
}
pub(super) async fn lock_scope(
    tx: &mut Transaction<'_, Postgres>,
    scope: &str,
) -> Result<i64, ViewerStoreError> {
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
async fn lock_viewers(
    tx: &mut Transaction<'_, Postgres>,
    s: &str,
    source: Uuid,
    target: Uuid,
) -> Result<(), ViewerStoreError> {
    if source == target {
        return Err(invalid());
    }
    let rows=sqlx::query("SELECT id FROM viewers WHERE scope_id=$1 AND id=ANY($2) AND merged_into IS NULL ORDER BY id FOR UPDATE").bind(s).bind(vec![source,target]).fetch_all(&mut **tx).await.map_err(database_error)?;
    if rows.len() != 2 {
        return Err(invalid());
    }
    sqlx::query("SELECT viewer_id FROM viewer_companionship WHERE scope_id=$1 AND viewer_id=ANY($2) ORDER BY viewer_id FOR UPDATE").bind(s).bind(vec![source,target]).fetch_all(&mut **tx).await.map_err(database_error)?;
    Ok(())
}
async fn summary(
    tx: &mut Transaction<'_, Postgres>,
    s: &str,
    v: Uuid,
) -> Result<MergeViewerSummary, ViewerStoreError> {
    let row=sqlx::query("SELECT v.current_alias,COALESCE(c.familiarity_milli,0) AS familiar,COALESCE(c.affinity_milli,0) AS affinity FROM viewers v LEFT JOIN viewer_companionship c ON c.scope_id=v.scope_id AND c.viewer_id=v.id WHERE v.scope_id=$1 AND v.id=$2").bind(s).bind(v).fetch_one(&mut **tx).await.map_err(database_error)?;
    let mut counts = Vec::new();
    for table in ["viewer_identities", "viewer_events", "memories"] {
        counts.push(
            sqlx::query_scalar::<_, i64>(&format!(
                "SELECT COUNT(*) FROM {table} WHERE scope_id=$1 AND viewer_id=$2"
            ))
            .bind(s)
            .bind(v)
            .fetch_one(&mut **tx)
            .await
            .map_err(database_error)?,
        );
    }
    if counts[0] == 0 {
        return Err(invalid());
    }
    let relations:i64=sqlx::query_scalar("SELECT COUNT(*) FROM relationship_facts WHERE scope_id=$1 AND ((source_kind='viewer' AND source_id=$2) OR (target_kind='viewer' AND target_id=$2))").bind(s).bind(v.to_string()).fetch_one(&mut **tx).await.map_err(database_error)?;
    Ok(MergeViewerSummary {
        viewer_id: v.to_string(),
        alias: row.get("current_alias"),
        identities: counts[0],
        events: counts[1],
        memories: counts[2],
        relationships: relations,
        familiarity_milli: row.get("familiar"),
        affinity_milli: row.get("affinity"),
    })
}
async fn preview(
    tx: &mut Transaction<'_, Postgres>,
    s: &str,
    source: Uuid,
    target: Uuid,
    revision: i64,
) -> Result<ViewerMergePreview, ViewerStoreError> {
    lock_viewers(tx, s, source, target).await?;
    let source_info = summary(tx, s, source).await?;
    let target_info = summary(tx, s, target).await?;
    let mut digest = Sha256::new();
    digest.update(format!("{s}:{source}:{target}:{revision}"));
    for table in [
        "viewer_identities",
        "viewer_events",
        "viewer_aliases",
        "viewer_companionship",
        "viewer_presence",
        "viewer_observed_sessions",
        "gift_ledger",
        "affinity_ledger",
        "companionship_reply_events",
        "companionship_completed_events",
        "companionship_daily_chats",
        "memories",
        "memory_jobs",
        "memory_suppressions",
    ] {
        let checksum:String=sqlx::query_scalar(&format!("SELECT COALESCE(string_agg(h,',' ORDER BY h),'') FROM (SELECT md5(to_jsonb(t)::text) h FROM {table} t WHERE scope_id=$1 AND viewer_id=ANY($2)) rows")).bind(s).bind(vec![source,target]).fetch_one(&mut **tx).await.map_err(database_error)?;
        digest.update(table);
        digest.update(checksum);
    }
    let checksum:String=sqlx::query_scalar("SELECT COALESCE(string_agg(h,',' ORDER BY h),'') FROM(SELECT md5(to_jsonb(t)::text) h FROM relationship_facts t WHERE scope_id=$1 AND ((source_kind='viewer' AND source_id=ANY($2)) OR(target_kind='viewer' AND target_id=ANY($2)))) rows").bind(s).bind(vec![source.to_string(),target.to_string()]).fetch_one(&mut **tx).await.map_err(database_error)?;
    digest.update(checksum);
    digest.update(format!("{:?}{:?}", source_info.alias, target_info.alias));
    let days: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT day) FROM viewer_presence WHERE scope_id=$1 AND viewer_id=ANY($2)",
    )
    .bind(s)
    .bind(vec![source, target])
    .fetch_one(&mut **tx)
    .await
    .map_err(database_error)?;
    Ok(ViewerMergePreview{resulting_familiarity_milli:(days*1000).min(100000)as i32,resulting_affinity_milli:source_info.affinity_milli.max(target_info.affinity_milli),source:source_info,target:target_info,revision,fingerprint:format!("{:x}",digest.finalize()),risks:vec!["熟悉度按来访日期并集；好感取两档案较大值，不相加，互补历史可能低估；历史流水保留并写差额调节；合并前流水不可直接撤销，纠正须提交有审计理由的人工调整。".into(),"重复记忆键将冻结并退出自动召回，需管理员纠正；关系改为单方声称待重新确认。".into(),"源身份永久指向目标；昵称相同不构成身份依据；旧生成与后台租约全部失效。".into()]})
}
impl ViewerMergeStore for PostgresViewerEventStore {
    fn preview_merge<'a>(
        &'a self,
        s: &'a str,
        a: &'a str,
        b: &'a str,
    ) -> ViewerStoreFuture<'a, ViewerMergePreview> {
        Box::pin(async move {
            let mut tx = self.begin().await?;
            let revision = lock_scope(&mut tx, s).await?;
            let p = preview(&mut tx, s, id(a)?, id(b)?, revision).await?;
            tx.commit().await.map_err(database_error)?;
            Ok(p)
        })
    }
    fn apply_merge<'a>(
        &'a self,
        s: &'a str,
        r: &'a ViewerMergeRequest,
        n: i64,
    ) -> ViewerStoreFuture<'a, ViewerMergeOutcome> {
        Box::pin(async move {
            if r.reason.trim().is_empty()
                || r.reason.len() > 2048
                || r.actor.trim().is_empty()
                || r.actor.len() > 128
                || r.request_key.is_empty()
                || r.request_key.len() > 128
            {
                return Err(invalid());
            }
            let source = id(&r.source_viewer_id)?;
            let target = id(&r.target_viewer_id)?;
            let mut tx = self.begin().await?;
            let revision = lock_scope(&mut tx, s).await?;
            let fingerprint = format!("{:x}", Sha256::digest(format!("{r:?}")));
            if let Some(old)=sqlx::query("SELECT request_fingerprint,target_viewer_id,result_revision FROM viewer_merge_audit WHERE scope_id=$1 AND request_key=$2").bind(s).bind(&r.request_key).fetch_optional(&mut *tx).await.map_err(database_error)?{if old.get::<String,_>("request_fingerprint")!=fingerprint{return Err(invalid())}return Ok(ViewerMergeOutcome{canonical_viewer_id:old.get::<Uuid,_>("target_viewer_id").to_string(),revision:old.get("result_revision")})}
            if r.expected_revision != revision {
                return Err(invalid());
            }
            let p = preview(&mut tx, s, source, target, revision).await?;
            if p.fingerprint != r.fingerprint {
                return Err(invalid());
            }
            migrate::apply(&mut tx, s, source, target, &p, r, n).await?;
            // Memory invalidation triggers may already advance this scope while
            // moving conflicting records. Preserve that monotonic fence.
            let new_revision: i64 = sqlx::query_scalar(
                "UPDATE memory_scopes SET revision=revision+1 WHERE scope_id=$1 RETURNING revision",
            )
            .bind(s)
            .fetch_one(&mut *tx)
            .await
            .map_err(database_error)?;
            sqlx::query("INSERT INTO viewer_merge_audit(scope_id,request_key,source_viewer_id,target_viewer_id,expected_revision,preview_fingerprint,request_fingerprint,reason,actor,created_at_ms,result_revision) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)").bind(s).bind(&r.request_key).bind(source).bind(target).bind(revision).bind(&r.fingerprint).bind(fingerprint).bind(&r.reason).bind(&r.actor).bind(n).bind(new_revision).execute(&mut *tx).await.map_err(database_error)?;
            tx.commit().await.map_err(database_error)?;
            Ok(ViewerMergeOutcome {
                canonical_viewer_id: target.to_string(),
                revision: new_revision,
            })
        })
    }
}
