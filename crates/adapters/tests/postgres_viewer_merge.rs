use meowlive_adapters::storage::postgres::PostgresViewerEventStore;
use meowlive_application::ports::{
    companionship::CompanionshipStore, viewer_merge::*, viewers::ViewerEventStore,
};
use meowlive_domain::event::*;
fn event(id: &str, identity: &str) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "bilibili".into(),
        viewer: "同名".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "merge-test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: identity.into(),
        }),
        occurred_at_ms: 1_700_000_000_000,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: format!("body-{id}"),
        },
    }
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn merge_is_explicit_audited_fenced_and_routes_new_identity_events() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("merge-test-{}", uuid::Uuid::new_v4());
    let a = event("a", "identity-a");
    let b = event("b", "identity-b");
    let out = store
        .accept_events(&scope, "s", &[a.clone(), b.clone()], a.occurred_at_ms)
        .await
        .unwrap();
    let source = out[0].viewer_id.clone().unwrap();
    let target = out[1].viewer_id.clone().unwrap();
    assert_ne!(source, target);
    assert!(
        store
            .preview_merge("wrong-scope", &source, &target)
            .await
            .is_err()
    );
    let preview = store.preview_merge(&scope, &source, &target).await.unwrap();
    assert_eq!(preview.resulting_familiarity_milli, 1000);
    let request = ViewerMergeRequest {
        source_viewer_id: source.clone(),
        target_viewer_id: target.clone(),
        expected_revision: preview.revision,
        fingerprint: preview.fingerprint,
        request_key: "merge".into(),
        reason: "verified same person".into(),
        actor: "admin".into(),
    };
    let result = store
        .apply_merge(&scope, &request, a.occurred_at_ms as i64 + 1)
        .await
        .unwrap();
    assert_eq!(result.canonical_viewer_id, target);
    assert_eq!(
        store
            .apply_merge(&scope, &request, a.occurred_at_ms as i64 + 2)
            .await
            .unwrap(),
        result
    );
    let mut conflict = request.clone();
    conflict.reason = "different".into();
    assert!(
        store
            .apply_merge(&scope, &conflict, a.occurred_at_ms as i64 + 3)
            .await
            .is_err()
    );
    let c = event("c", "identity-a");
    let routed = store
        .accept_events(&scope, "s", &[c], a.occurred_at_ms + 4)
        .await
        .unwrap();
    assert_eq!(routed[0].viewer_id.as_deref(), Some(target.as_str()));
    let detail = store.detail(&scope, &target, 100).await.unwrap().unwrap();
    assert_eq!(detail.familiarity_milli, 1000);
    assert!(detail.ledger.iter().any(|l| l.kind == "merge_familiarity"));
}
use meowlive_application::ports::{memory::EmbeddingBatch, memory_store::MemoryStore};
use meowlive_domain::memory::*;
fn request(p: ViewerMergePreview, key: &str) -> ViewerMergeRequest {
    ViewerMergeRequest {
        source_viewer_id: p.source.viewer_id,
        target_viewer_id: p.target.viewer_id,
        expected_revision: p.revision,
        fingerprint: p.fingerprint,
        request_key: key.into(),
        reason: "verified identical person".into(),
        actor: "admin".into(),
    }
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn stale_preview_and_concurrent_ingest_never_partially_merge() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("merge-stale-{}", uuid::Uuid::new_v4());
    let a = event("a", "a");
    let b = event("b", "b");
    let out = store
        .accept_events(&scope, "s", &[a.clone(), b], a.occurred_at_ms)
        .await
        .unwrap();
    let source = out[0].viewer_id.clone().unwrap();
    let target = out[1].viewer_id.clone().unwrap();
    let r = request(
        store.preview_merge(&scope, &source, &target).await.unwrap(),
        "stale",
    );
    store
        .accept_events(&scope, "s", &[event("new", "a")], a.occurred_at_ms)
        .await
        .unwrap();
    assert!(
        store
            .apply_merge(&scope, &r, a.occurred_at_ms as i64 + 1)
            .await
            .is_err()
    );
    let r = request(
        store.preview_merge(&scope, &source, &target).await.unwrap(),
        "race",
    );
    let input = [event("racing", "a")];
    let (merge, ingest) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            store.apply_merge(&scope, &r, a.occurred_at_ms as i64 + 2),
            store.accept_events(&scope, "s", &input, a.occurred_at_ms + 2)
        )
    })
    .await
    .unwrap();
    assert!(ingest.is_ok());
    if merge.is_err() {
        let r = request(
            store.preview_merge(&scope, &source, &target).await.unwrap(),
            "race-retry",
        );
        store
            .apply_merge(&scope, &r, a.occurred_at_ms as i64 + 3)
            .await
            .unwrap();
    }
    assert_eq!(
        store
            .accept_events(&scope, "s", &[event("last", "a")], a.occurred_at_ms + 4)
            .await
            .unwrap()[0]
            .viewer_id
            .as_deref(),
        Some(target.as_str())
    );
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn merge_preserves_evidence_rewards_and_invalidates_old_jobs() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("merge-facts-{}", uuid::Uuid::new_v4());
    let t = 1_700_000_000_000;
    let mut a = event("a", "a");
    a.kind = EventKind::Chat {
        text: "我喜欢猫".into(),
    };
    let mut b = event("b", "b");
    b.kind = EventKind::Chat {
        text: "我喜欢狗".into(),
    };
    let out = store
        .accept_events(&scope, "s", &[a.clone(), b.clone()], t as u64)
        .await
        .unwrap();
    let source = out[0].viewer_id.clone().unwrap();
    let target = out[1].viewer_id.clone().unwrap();
    for e in [&a, &b] {
        store
            .register_reply(&scope, &e.id, std::slice::from_ref(e), t as u64)
            .await
            .unwrap();
        store.complete_reply(&scope, &e.id, t as u64).await.unwrap();
    }
    for _ in 0..2 {
        let job = store.claim_job(&scope, t).await.unwrap().unwrap();
        let value = if job.sources[0].text.contains('猫') {
            "猫"
        } else {
            "狗"
        };
        let c = MemoryCandidate {
            key: "preference".into(),
            value: value.into(),
            kind: MemoryKind::Preference,
            evidence: vec![Evidence::from_source(&job.sources[0], &job.sources[0].text)],
            explicit: true,
            confidence: 1.0,
            valid_until_ms: None,
        };
        store.finish_job(&scope, &job, &[c], t + 1).await.unwrap();
    }
    let embedding = store
        .claim_embedding(&scope, "m", 2, t + 2)
        .await
        .unwrap()
        .unwrap();
    store
        .accept_events(&scope, "s", &[event("pending", "a")], t as u64 + 2)
        .await
        .unwrap();
    let pending = store.claim_job(&scope, t + 2).await.unwrap().unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let fact = uuid::Uuid::new_v4();
    let old_token = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO relationship_facts(scope_id,id,source_kind,source_id,target_kind,target_id,kind,confirmation,evidence,created_at_ms,updated_at_ms) VALUES($1,$2,'viewer',$3,'viewer',$4,'acquaintance','confirmed','[]',$5,$5)").bind(&scope).bind(fact).bind(&source).bind(&target).bind(t).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO relationship_outbox(scope_id,fact_id,version,state,available_at_ms,created_at_ms,token,lease_until_ms) VALUES($1,$2,1,'running',$3,$3,$4,$3+60000)").bind(&scope).bind(fact).bind(t).bind(old_token).execute(&pool).await.unwrap();
    let preview = store.preview_merge(&scope, &source, &target).await.unwrap();
    assert_eq!(preview.resulting_affinity_milli, 200);
    store
        .apply_merge(&scope, &request(preview, "merge"), t + 3)
        .await
        .unwrap();
    let records = store.list(&scope, &target, 20, t + 4).await.unwrap();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|r| r.locked
        && r.status == MemoryStatus::Candidate
        && r.candidate.evidence.len() == 1));
    assert!(
        store
            .finish_job(&scope, &pending, &[], t + 4)
            .await
            .is_err()
    );
    assert!(
        store
            .store_embedding(
                &scope,
                &embedding,
                &EmbeddingBatch {
                    model: "m".into(),
                    dimensions: 2,
                    vectors: vec![vec![1.0, 0.0]]
                },
                t + 4
            )
            .await
            .is_err()
    );
    let detail = store.detail(&scope, &target, 100).await.unwrap().unwrap();
    assert_eq!(detail.affinity_milli, 200);
    assert_eq!(
        detail
            .ledger
            .iter()
            .filter(|l| l.kind == "exchange")
            .count(),
        2
    );
    let original: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM affinity_ledger WHERE scope_id=$1 AND original_viewer_id=$2",
    )
    .bind(&scope)
    .bind(uuid::Uuid::parse_str(&source).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(original >= 2);
    let deleted:bool=sqlx::query_scalar("SELECT deleted FROM relationship_facts WHERE scope_id=$1 AND id=$2 AND source_id=$3 AND target_id=$3").bind(&scope).bind(fact).bind(&target).fetch_one(&pool).await.unwrap();
    assert!(deleted);
    let invalidated:bool=sqlx::query_scalar("SELECT token IS NULL AND version=2 AND state='pending' FROM relationship_outbox WHERE scope_id=$1 AND fact_id=$2").bind(&scope).bind(fact).fetch_one(&pool).await.unwrap();
    assert!(invalidated);
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn old_negative_ledger_cannot_reverse_after_merge_but_new_adjustment_can() {
    use meowlive_application::ports::companionship::{AffinityAdjustment, AffinityReversal};
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("merge-reverse-{}", uuid::Uuid::new_v4());
    let t = 1_700_000_000_000;
    let out = store
        .accept_events(&scope, "s", &[event("a", "a"), event("b", "b")], t)
        .await
        .unwrap();
    let a = out[0].viewer_id.clone().unwrap();
    let b = out[1].viewer_id.clone().unwrap();
    for (viewer, key, delta) in [(&a, "plus", 1000), (&b, "target", 1000)] {
        store
            .adjust(
                &scope,
                viewer,
                &AffinityAdjustment {
                    request_key: key.into(),
                    reason: "manual".into(),
                    delta_milli: delta,
                },
                t,
            )
            .await
            .unwrap();
    }
    let old = store
        .adjust(
            &scope,
            &a,
            &AffinityAdjustment {
                request_key: "minus".into(),
                reason: "manual".into(),
                delta_milli: -500,
            },
            t,
        )
        .await
        .unwrap();
    let p = store.preview_merge(&scope, &a, &b).await.unwrap();
    store
        .apply_merge(&scope, &request(p, "merge"), t as i64 + 1)
        .await
        .unwrap();
    assert!(
        store
            .reverse(
                &scope,
                &b,
                &AffinityReversal {
                    request_key: "old-reverse".into(),
                    reason: "undo".into(),
                    ledger_id: old.clone()
                },
                t + 2
            )
            .await
            .is_err()
    );
    let detail = store.detail(&scope, &b, 100).await.unwrap().unwrap();
    assert_eq!(detail.affinity_milli, 1000);
    assert!(
        !detail
            .ledger
            .iter()
            .find(|r| r.ledger_id == old)
            .unwrap()
            .reversible
    );
    let fresh = store
        .adjust(
            &scope,
            &b,
            &AffinityAdjustment {
                request_key: "fresh".into(),
                reason: "manual".into(),
                delta_milli: -200,
            },
            t + 3,
        )
        .await
        .unwrap();
    assert!(
        store
            .detail(&scope, &b, 100)
            .await
            .unwrap()
            .unwrap()
            .ledger
            .iter()
            .find(|r| r.ledger_id == fresh)
            .unwrap()
            .reversible
    );
    store
        .reverse(
            &scope,
            &b,
            &AffinityReversal {
                request_key: "fresh-reverse".into(),
                reason: "undo".into(),
                ledger_id: fresh.clone(),
            },
            t + 4,
        )
        .await
        .unwrap();
    let detail = store.detail(&scope, &b, 100).await.unwrap().unwrap();
    assert_eq!(detail.affinity_milli, 1000);
    assert!(
        !detail
            .ledger
            .iter()
            .find(|r| r.ledger_id == fresh)
            .unwrap()
            .reversible
    );
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn concurrent_merge_and_completed_reward_have_bounded_consistent_lock_order() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("merge-complete-{}", uuid::Uuid::new_v4());
    let a = event("a", "a");
    let b = event("b", "b");
    let t = a.occurred_at_ms;
    let out = store
        .accept_events(&scope, "s", &[a.clone(), b], t)
        .await
        .unwrap();
    let source = out[0].viewer_id.clone().unwrap();
    let target = out[1].viewer_id.clone().unwrap();
    store
        .register_reply(&scope, "speech", &[a], t)
        .await
        .unwrap();
    let r = request(
        store.preview_merge(&scope, &source, &target).await.unwrap(),
        "merge",
    );
    let (merged, completed) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            store.apply_merge(&scope, &r, t as i64 + 1),
            store.complete_reply(&scope, "speech", t + 1)
        )
    })
    .await
    .unwrap();
    completed.unwrap();
    if merged.is_err() {
        let fresh = request(
            store.preview_merge(&scope, &source, &target).await.unwrap(),
            "retry",
        );
        store
            .apply_merge(&scope, &fresh, t as i64 + 2)
            .await
            .unwrap();
    }
    assert_eq!(
        store
            .detail(&scope, &target, 100)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        200
    );
}
