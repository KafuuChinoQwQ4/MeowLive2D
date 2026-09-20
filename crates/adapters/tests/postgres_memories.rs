use meowlive_adapters::storage::postgres::PostgresViewerEventStore;
use meowlive_application::ports::memory_store::MemoryStore;
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn empty_scope_has_no_context_or_jobs() {
    let Ok(url) = std::env::var("MEOWLIVE_TEST_DATABASE_URL") else {
        return;
    };
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-test-{}", uuid::Uuid::new_v4());
    assert_eq!(store.revision(&scope).await.unwrap(), 0);
    assert!(store.claim_job(&scope, 100).await.unwrap().is_none());
    assert!(
        store
            .context(&scope, &[], None, 100)
            .await
            .unwrap()
            .records
            .is_empty()
    );
}
use meowlive_application::ports::{
    memory::EmbeddingBatch, memory_store::*, viewers::ViewerEventStore,
};
use meowlive_domain::{event::*, memory::*};
fn event(id: &str, t: i64, text: &str) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "bilibili".into(),
        viewer: "猫".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "memory-test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "a".into(),
        }),
        occurred_at_ms: t as u64,
        gift_metadata: None,
        kind: EventKind::Chat { text: text.into() },
    }
}
fn candidate(job: &ExtractionJob, key: &str, value: &str, kind: MemoryKind) -> MemoryCandidate {
    MemoryCandidate {
        key: key.into(),
        value: value.into(),
        kind,
        evidence: vec![Evidence::from_source(&job.sources[0], &job.sources[0].text)],
        explicit: true,
        confidence: 1.0,
        valid_until_ms: None,
    }
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn lifecycle_leases_cross_day_vectors_and_admin_fence() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-lifecycle-{}", uuid::Uuid::new_v4());
    let t = 1_800_000_000_000;
    let a = event("a", t, "我养猫");
    let viewer = store
        .accept_events(&scope, "session", &[a.clone()], t as u64)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let job = store.claim_job(&scope, t).await.unwrap().unwrap();
    assert!(store.claim_job(&scope, t + 1).await.unwrap().is_none());
    let c = candidate(&job, "stable_fact", "猫", MemoryKind::StableFact);
    store.finish_job(&scope, &job, &[c], t + 1).await.unwrap();
    assert_eq!(
        store.list(&scope, &viewer, 50, t + 1).await.unwrap()[0].status,
        MemoryStatus::Candidate
    );
    assert!(
        store
            .context(&scope, &[viewer.clone()], None, t + 1)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    assert!(store.finish_job(&scope, &job, &[], t + 2).await.is_err());
    let b = event("b", t + DAY_MS, "我养猫");
    store
        .accept_events(&scope, "session", &[b], (t + DAY_MS) as u64)
        .await
        .unwrap();
    let job = store.claim_job(&scope, t + DAY_MS).await.unwrap().unwrap();
    let c = candidate(&job, "stable_fact", "猫", MemoryKind::StableFact);
    store
        .finish_job(&scope, &job, &[c], t + DAY_MS + 1)
        .await
        .unwrap();
    let record = store
        .list(&scope, &viewer, 50, t + DAY_MS + 1)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(record.status, MemoryStatus::LongTerm);
    assert_eq!(record.candidate.evidence.len(), 2);
    let embedding = store
        .claim_embedding(&scope, "model", 2, t + DAY_MS + 2)
        .await
        .unwrap()
        .unwrap();
    store
        .store_embedding(
            &scope,
            &embedding,
            &EmbeddingBatch {
                model: "model".into(),
                dimensions: 2,
                vectors: vec![vec![0.5, 1.0]],
            },
            t + DAY_MS + 3,
        )
        .await
        .unwrap();
    let snapshot = store
        .context(
            &scope,
            &[viewer.clone()],
            Some(&QueryEmbedding {
                model: "model".into(),
                vector: vec![0.5, 1.0],
            }),
            t + DAY_MS + 4,
        )
        .await
        .unwrap();
    assert_eq!(snapshot.records.len(), 1);
    let pending = event("pending", t + DAY_MS + 5, "我养猫");
    store
        .accept_events(&scope, "session", &[pending], (t + DAY_MS + 5) as u64)
        .await
        .unwrap();
    let oldjob = store
        .claim_job(&scope, t + DAY_MS + 5)
        .await
        .unwrap()
        .unwrap();
    let request = MemoryAdminRequest {
        viewer_id: viewer.clone(),
        memory_id: record.id.clone(),
        expected_version: record.version,
        request_key: "delete".into(),
        reason: "wrong fact".into(),
        actor: "admin".into(),
    };
    store
        .admin_delete(&scope, &request, t + DAY_MS + 6)
        .await
        .unwrap();
    store
        .admin_delete(&scope, &request, t + DAY_MS + 7)
        .await
        .unwrap();
    assert!(
        store
            .admin_delete("wrong-scope", &request, t + DAY_MS + 7)
            .await
            .is_err()
    );
    let c = candidate(&oldjob, "stable_fact", "猫", MemoryKind::StableFact);
    store
        .finish_job(&scope, &oldjob, &[c], t + DAY_MS + 8)
        .await
        .unwrap();
    assert!(
        store
            .context(&scope, &[viewer.clone()], None, t + DAY_MS + 8)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    assert!(
        store
            .store_embedding(
                &scope,
                &embedding,
                &EmbeddingBatch {
                    model: "model".into(),
                    dimensions: 2,
                    vectors: vec![vec![0.5, 1.0]]
                },
                t + DAY_MS + 9
            )
            .await
            .is_err()
    );
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    for table in [
        "memory_embedding_jobs",
        "memory_vectors",
        "memory_evidence",
        "memory_audit",
        "memory_suppressions",
        "memory_tombstones",
        "memories",
        "memory_jobs",
        "memory_scopes",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE scope_id=$1"))
            .bind(&scope)
            .execute(&pool)
            .await
            .unwrap();
    }
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn retries_expiry_and_freeze_are_persistent() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-retries-{}", uuid::Uuid::new_v4());
    let t = 1_800_000_000_000;
    let viewer = store
        .accept_events(&scope, "s", &[event("a", t, "我喜欢猫")], t as u64)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let first = store.claim_job(&scope, t).await.unwrap().unwrap();
    let second = store.claim_job(&scope, t + 60001).await.unwrap().unwrap();
    assert_ne!(first.token, second.token);
    assert!(
        store
            .finish_job(&scope, &first, &[], t + 60002)
            .await
            .is_err()
    );
    store.fail_job(&scope, &second, t + 60002).await.unwrap();
    assert!(store.claim_job(&scope, t + 60003).await.unwrap().is_none());
    let third = store.claim_job(&scope, t + 65000).await.unwrap().unwrap();
    assert_eq!(third.attempts, 3);
    store.fail_job(&scope, &third, t + 65001).await.unwrap();
    assert_eq!(store.status(&scope).await.unwrap().failed, 1);
    store
        .accept_events(
            &scope,
            "s",
            &[event("b", t + 70000, "我今天很累")],
            (t + 70000) as u64,
        )
        .await
        .unwrap();
    let job = store.claim_job(&scope, t + 70000).await.unwrap().unwrap();
    let c = candidate(&job, "temporary_state", "累", MemoryKind::TemporaryState);
    store
        .finish_job(&scope, &job, &[c], t + 70001)
        .await
        .unwrap();
    let record = store
        .list(&scope, &viewer, 10, t + 70001)
        .await
        .unwrap()
        .remove(0);
    let request = MemoryAdminRequest {
        viewer_id: viewer.clone(),
        memory_id: record.id,
        expected_version: record.version,
        request_key: "freeze".into(),
        reason: "manual freeze".into(),
        actor: "admin".into(),
    };
    store
        .admin_freeze(&scope, &request, true, t + 70002)
        .await
        .unwrap();
    assert!(store.list(&scope, &viewer, 10, t + 70003).await.unwrap()[0].locked);
    assert!(
        store
            .context(&scope, &[viewer], None, t + 70000 + DAY_MS)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    store.maintenance(&scope, t + 31 * DAY_MS).await.unwrap();
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn evidence_quote_cannot_be_promoted_by_losing_original_context() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-quote-{}", uuid::Uuid::new_v4());
    let t = 1_800_000_000_000;
    let viewer = store
        .accept_events(
            &scope,
            "s",
            &[event("a", t, "如果我喜欢猫会怎么样")],
            t as u64,
        )
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let job = store.claim_job(&scope, t).await.unwrap().unwrap();
    let mut c = candidate(&job, "preference", "猫", MemoryKind::Preference);
    c.evidence[0].quote = "我喜欢猫".into();
    store.finish_job(&scope, &job, &[c], t + 1).await.unwrap();
    assert_eq!(
        store.list(&scope, &viewer, 10, t + 1).await.unwrap()[0].status,
        MemoryStatus::Candidate
    );
    assert!(
        store
            .context(&scope, &[viewer], None, t + 1)
            .await
            .unwrap()
            .records
            .is_empty()
    );
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn resolve_viewers_preserves_order_duplicates_and_unknown_positions() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-resolve-{}", uuid::Uuid::new_v4());
    let e = event("known", 1_800_000_000_000, "我喜欢猫");
    let viewer = store
        .accept_events(&scope, "s", &[e.clone()], e.occurred_at_ms)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let unknown = event("unknown", 1_800_000_000_000, "我喜欢猫");
    assert_eq!(
        store
            .resolve_viewers(&scope, &[e.clone(), unknown, e])
            .await
            .unwrap(),
        vec![viewer.clone(), String::new(), viewer]
    );
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn maintenance_operations_are_audited_idempotent_and_invalidate_old_vectors() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-ops-{}", uuid::Uuid::new_v4());
    let t = 1_800_000_000_000;
    store
        .accept_events(&scope, "s", &[event("a", t, "我喜欢猫")], t as u64)
        .await
        .unwrap();
    for offset in [0, 2000, 7000] {
        let job = store.claim_job(&scope, t + offset).await.unwrap().unwrap();
        store.fail_job(&scope, &job, t + offset).await.unwrap();
    }
    assert_eq!(store.status(&scope).await.unwrap().failed, 1);
    let req = MemoryMaintenanceRequest {
        request_key: "retry".into(),
        reason: "service recovered".into(),
        actor: "admin".into(),
    };
    store.retry_failed(&scope, &req, t + 10000).await.unwrap();
    store.retry_failed(&scope, &req, t + 10001).await.unwrap();
    let job = store.claim_job(&scope, t + 10002).await.unwrap().unwrap();
    let c = candidate(&job, "preference", "猫", MemoryKind::Preference);
    store
        .finish_job(&scope, &job, &[c], t + 10003)
        .await
        .unwrap();
    let embed = store
        .claim_embedding(&scope, "m", 2, t + 10004)
        .await
        .unwrap()
        .unwrap();
    let rebuild = MemoryMaintenanceRequest {
        request_key: "rebuild".into(),
        ..req.clone()
    };
    let revision = store.revision(&scope).await.unwrap();
    store
        .rebuild_vectors(&scope, &rebuild, t + 10005)
        .await
        .unwrap();
    assert_eq!(store.revision(&scope).await.unwrap(), revision);
    assert!(
        store
            .store_embedding(
                &scope,
                &embed,
                &EmbeddingBatch {
                    model: "m".into(),
                    dimensions: 2,
                    vectors: vec![vec![1.0, 0.0]]
                },
                t + 10006
            )
            .await
            .is_err()
    );
    assert!(
        store
            .rebuild_vectors(&scope, &req, t + 10007)
            .await
            .is_err()
    );
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn unfreeze_allows_new_evidence_but_keeps_original_sources_suppressed() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-unfreeze-{}", uuid::Uuid::new_v4());
    let t = 1_800_000_000_000;
    let viewer = store
        .accept_events(&scope, "s", &[event("a", t, "我喜欢猫")], t as u64)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let job = store.claim_job(&scope, t).await.unwrap().unwrap();
    store
        .finish_job(
            &scope,
            &job,
            &[candidate(&job, "preference", "猫", MemoryKind::Preference)],
            t + 1,
        )
        .await
        .unwrap();
    let record = store
        .list(&scope, &viewer, 10, t + 1)
        .await
        .unwrap()
        .remove(0);
    let mut req = MemoryAdminRequest {
        viewer_id: viewer.clone(),
        memory_id: record.id,
        expected_version: record.version,
        request_key: "correct".into(),
        reason: "correction".into(),
        actor: "admin".into(),
    };
    store
        .admin_correct(&scope, &req, "狗", t + 2)
        .await
        .unwrap();
    req.request_key = "unfreeze".into();
    req.expected_version += 1;
    store
        .admin_freeze(&scope, &req, false, t + 3)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query("UPDATE memory_jobs SET state='pending',body='我喜欢猫',attempts=0 WHERE scope_id=$1 AND event_id='a'").bind(&scope).execute(&pool).await.unwrap();
    let old = store.claim_job(&scope, t + 4).await.unwrap().unwrap();
    store
        .finish_job(
            &scope,
            &old,
            &[candidate(&old, "preference", "猫", MemoryKind::Preference)],
            t + 5,
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .context(&scope, &[viewer.clone()], None, t + 5)
            .await
            .unwrap()
            .records[0]
            .candidate
            .value,
        "狗"
    );
    store
        .accept_events(
            &scope,
            "s",
            &[event("same-correction", t + 6, "我喜欢狗")],
            (t + 6) as u64,
        )
        .await
        .unwrap();
    let same = store.claim_job(&scope, t + 6).await.unwrap().unwrap();
    store
        .finish_job(
            &scope,
            &same,
            &[candidate(&same, "preference", "狗", MemoryKind::Preference)],
            t + 7,
        )
        .await
        .unwrap();
    store
        .accept_events(
            &scope,
            "s",
            &[event("b", t + 6, "我喜欢兔")],
            (t + 6) as u64,
        )
        .await
        .unwrap();
    let new = store.claim_job(&scope, t + 6).await.unwrap().unwrap();
    store
        .finish_job(
            &scope,
            &new,
            &[candidate(&new, "preference", "兔", MemoryKind::Preference)],
            t + 7,
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .context(&scope, &[viewer], None, t + 7)
            .await
            .unwrap()
            .records[0]
            .candidate
            .value,
        "兔"
    );
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn retrieval_combines_half_life_confirmation_and_vector_distance() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-rank-{}", uuid::Uuid::new_v4());
    let t = 1_800_000_000_000;
    let mut viewer = String::new();
    for (eventid, at, text, key, value, kind) in [
        (
            "a",
            t,
            "我喜欢猫",
            "preference",
            "猫",
            MemoryKind::Preference,
        ),
        (
            "b",
            t + DAY_MS,
            "我今天去了公园",
            "experience",
            "公园",
            MemoryKind::Experience,
        ),
    ] {
        viewer = store
            .accept_events(&scope, "s", &[event(eventid, at, text)], at as u64)
            .await
            .unwrap()[0]
            .viewer_id
            .clone()
            .unwrap();
        let job = store.claim_job(&scope, at).await.unwrap().unwrap();
        store
            .finish_job(&scope, &job, &[candidate(&job, key, value, kind)], at + 1)
            .await
            .unwrap();
    }
    let snapshot = store
        .context(&scope, &[viewer.clone()], None, t + 8 * DAY_MS)
        .await
        .unwrap();
    assert_eq!(snapshot.records[0].candidate.value, "猫");
    for _ in 0..2 {
        let j = store
            .claim_embedding(&scope, "m", 2, t + 8 * DAY_MS)
            .await
            .unwrap()
            .unwrap();
        let vector = if j.text == "猫" {
            vec![2.0, 0.0]
        } else {
            vec![0.0, 0.0]
        };
        store
            .store_embedding(
                &scope,
                &j,
                &EmbeddingBatch {
                    model: "m".into(),
                    dimensions: 2,
                    vectors: vec![vector],
                },
                t + 8 * DAY_MS + 1,
            )
            .await
            .unwrap();
    }
    let snapshot = store
        .context(
            &scope,
            &[viewer],
            Some(&QueryEmbedding {
                model: "m".into(),
                vector: vec![0.0, 0.0],
            }),
            t + 8 * DAY_MS + 2,
        )
        .await
        .unwrap();
    assert_eq!(snapshot.records[0].candidate.value, "公园");
    let viewer = snapshot.records[0].viewer_id.clone();
    let aged = store
        .context(
            &scope,
            &[viewer],
            Some(&QueryEmbedding {
                model: "m".into(),
                vector: vec![0.0, 0.0],
            }),
            t + 15 * DAY_MS,
        )
        .await
        .unwrap();
    assert_eq!(aged.records[0].candidate.value, "猫");
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn retry_resets_only_exhausted_embedding_leases() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-embed-retry-{}", uuid::Uuid::new_v4());
    let t = 1_700_000_000_000;
    store
        .accept_events(&scope, "s", &[event("a", t, "我喜欢猫")], t as u64)
        .await
        .unwrap();
    let job = store.claim_job(&scope, t).await.unwrap().unwrap();
    store
        .finish_job(
            &scope,
            &job,
            &[candidate(&job, "preference", "猫", MemoryKind::Preference)],
            t + 1,
        )
        .await
        .unwrap();
    for n in [t + 2, t + 60003, t + 120004] {
        assert!(
            store
                .claim_embedding(&scope, "m", 2, n)
                .await
                .unwrap()
                .is_some()
        );
    }
    assert!(
        store
            .claim_embedding(&scope, "m", 2, t + 180005)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(store.status(&scope).await.unwrap().embedding_failed, 1);
    let req = MemoryMaintenanceRequest {
        request_key: "retry".into(),
        reason: "recovered".into(),
        actor: "admin".into(),
    };
    store.retry_failed(&scope, &req, t + 180006).await.unwrap();
    let active = store
        .claim_embedding(&scope, "m", 2, t + 180007)
        .await
        .unwrap()
        .unwrap();
    let req2 = MemoryMaintenanceRequest {
        request_key: "retry2".into(),
        ..req
    };
    store.retry_failed(&scope, &req2, t + 180008).await.unwrap();
    store
        .store_embedding(
            &scope,
            &active,
            &EmbeddingBatch {
                model: "m".into(),
                dimensions: 2,
                vectors: vec![vec![1.0, 0.0]],
            },
            t + 180009,
        )
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn maintenance_removes_old_scoring_bodies_but_keeps_dedup() {
    use meowlive_application::ports::companionship::CompanionshipStore;
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("memory-raw-clean-{}", uuid::Uuid::new_v4());
    let t = 1_700_000_000_000;
    let e = event("a", t, "retained private body");
    store
        .accept_events(&scope, "s", &[e.clone()], t as u64)
        .await
        .unwrap();
    store
        .register_reply(&scope, "completed", &[e.clone()], t as u64)
        .await
        .unwrap();
    store
        .complete_reply(&scope, "completed", t as u64)
        .await
        .unwrap();
    store
        .register_reply(&scope, "incomplete", &[e], t as u64)
        .await
        .unwrap();
    store.maintenance(&scope, t + 31 * DAY_MS).await.unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM companionship_reply_events WHERE scope_id=$1 AND chat_body IS NOT NULL").bind(&scope).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM companionship_daily_chats WHERE scope_id=$1")
            .bind(&scope)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}
