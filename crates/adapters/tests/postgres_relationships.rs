use meowlive_adapters::storage::postgres::PostgresViewerEventStore;
use meowlive_application::ports::{relationships::*, viewers::ViewerEventStore};
use meowlive_domain::event::*;
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn scoped_relationships_outbox_and_tombstones() {
    let s =
        PostgresViewerEventStore::connect(&std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
    let scope = format!("relationship-{}", uuid::Uuid::new_v4());
    let now = 1700000000000;
    let mut events = Vec::new();
    for n in 0..2 {
        events.push(LiveEvent {
            id: format!("e{n}"),
            source: "bilibili".into(),
            viewer: format!("user{n}"),
            viewer_identity: Some(ViewerIdentity {
                namespace: "test".into(),
                kind: ViewerIdentityKind::OpenId,
                external_id: n.to_string(),
            }),
            occurred_at_ms: now,
            gift_metadata: None,
            kind: EventKind::Chat {
                text: "我们是朋友".into(),
            },
        });
    }
    let saved = s.accept_events(&scope, "one", &events, now).await.unwrap();
    let a = saved[0].viewer_id.clone().unwrap();
    let b = saved[1].viewer_id.clone().unwrap();
    let input = RelationInput {
        source: RelationEntity {
            kind: EntityKind::Viewer,
            id: a.clone(),
        },
        target: RelationEntity {
            kind: EntityKind::Viewer,
            id: b.clone(),
        },
        kind: RelationKind::Friend,
        evidence: vec![RelationEvidence {
            source: "bilibili".into(),
            event_id: "e0".into(),
            quote: "我们是朋友".into(),
        }],
        expires_at_ms: None,
        admin_confirmed: false,
        reason: "用户自述".into(),
        request_key: "first".into(),
    };
    let f = s.create(&scope, &input, now).await.unwrap();
    assert_eq!(f.confirmation, RelationConfirmation::Claimed);
    assert_eq!(s.create(&scope, &input, now).await.unwrap(), f);
    assert!(s.query(&scope, &a, 1, 20, now).await.unwrap().is_empty());
    let mut bad = input.clone();
    bad.evidence[0].quote = "虚构证据".into();
    bad.request_key = "bad".into();
    assert!(s.create(&scope, &bad, now).await.is_err());
    bad = input.clone();
    bad.reason = "changed".into();
    assert!(s.create(&scope, &bad, now).await.is_err());
    assert!(s.create("other", &input, now).await.is_err());
    let confirmed = s
        .change(
            &scope,
            &f.id,
            1,
            RelationAction::Confirm,
            "人工核对",
            "confirm",
            now,
        )
        .await
        .unwrap();
    assert_eq!(confirmed.version, 2);
    assert_eq!(s.query(&scope, &a, 1, 20, now).await.unwrap().len(), 1);
    let lease = s.claim_outbox(&scope, now).await.unwrap().unwrap();
    assert_eq!(lease.projection.fact.version, 2);
    assert!(
        s.finish_outbox("other", &lease.id, &lease.token, now)
            .await
            .is_err()
    );
    s.fail_outbox(&scope, &lease.id, &lease.token, now)
        .await
        .unwrap();
    assert!(s.claim_outbox(&scope, now).await.unwrap().is_none());
    let lease = s.claim_outbox(&scope, now + 10000).await.unwrap().unwrap();
    let deleted = s
        .change(
            &scope,
            &f.id,
            2,
            RelationAction::Delete,
            "删除",
            "delete",
            now + 10000,
        )
        .await
        .unwrap();
    assert!(deleted.deleted);
    assert!(
        s.finish_outbox(&scope, &lease.id, &lease.token, now + 10000)
            .await
            .is_err()
    );
    assert!(
        s.validate_graph(
            &scope,
            &[GraphReference {
                fact_id: f.id.clone(),
                version: 2
            }],
            now
        )
        .await
        .unwrap()
        .is_empty()
    );
    let lease = s.claim_outbox(&scope, now + 10000).await.unwrap().unwrap();
    assert!(lease.projection.fact.deleted);
    s.finish_outbox(&scope, &lease.id, &lease.token, now + 10000)
        .await
        .unwrap();
    assert_eq!(
        s.rebuild(&scope, "rebuild", "重建", now + 10000)
            .await
            .unwrap(),
        1
    );
    let mut both = input;
    both.request_key = "both".into();
    both.evidence.push(RelationEvidence {
        source: "bilibili".into(),
        event_id: "e1".into(),
        quote: "我们是朋友".into(),
    });
    let f = s.create(&scope, &both, now).await.unwrap();
    assert_eq!(f.confirmation, RelationConfirmation::Claimed);
    assert!(s.query(&scope, &b, 2, 1, now).await.unwrap().is_empty());
    assert_eq!(s.list_admin(&scope, &b, 20, 0).await.unwrap().len(), 2);
    assert!(s.list_admin("other", &b, 20, 0).await.unwrap().is_empty());
    let pool = sqlx::PgPool::connect(&std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::query("UPDATE viewers SET merged_into=$3 WHERE scope_id=$1 AND id=$2")
        .bind(&scope)
        .bind(uuid::Uuid::parse_str(&a).unwrap())
        .bind(uuid::Uuid::parse_str(&b).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    both.request_key = "merged-endpoint".into();
    both.admin_confirmed = true;
    assert!(s.create(&scope, &both, now).await.is_err());
    assert!(s.query(&scope, &a, 3, 20, now).await.is_err());
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn memory_invalidations_tombstone_graph_in_same_transaction() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let s = PostgresViewerEventStore::connect(&url).await.unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let scope = format!("relationship-memory-{}", uuid::Uuid::new_v4());
    let now = 1700000000000u64;
    let event = LiveEvent {
        id: "e".into(),
        source: "bilibili".into(),
        viewer: "viewer".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "a".into(),
        }),
        occurred_at_ms: now,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: "我喜欢猫".into(),
        },
    };
    let viewer = s.accept_events(&scope, "one", &[event], now).await.unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    for (i, operation) in [
        "value='修正'",
        "deleted=true",
        "status='expired'",
        "expires_at_ms=1",
    ]
    .iter()
    .enumerate()
    {
        let memory = uuid::Uuid::new_v4();
        sqlx::query("INSERT INTO memories(id,scope_id,viewer_id,key,value,kind,status,created_at_ms,updated_at_ms) VALUES($1,$2,$3,$4,'猫','preference','long_term',$5,$5)").bind(memory).bind(&scope).bind(uuid::Uuid::parse_str(&viewer).unwrap()).bind(format!("key{i}")).bind(now as i64).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO memory_evidence(scope_id,memory_id,source,event_id,quote,occurred_at_ms,day) VALUES($1,$2,'bilibili','e','我喜欢猫',$3,1)").bind(&scope).bind(memory).bind(now as i64).execute(&pool).await.unwrap();
        let f = s
            .create(
                &scope,
                &RelationInput {
                    source: RelationEntity {
                        kind: EntityKind::Viewer,
                        id: viewer.clone(),
                    },
                    target: RelationEntity {
                        kind: EntityKind::Topic,
                        id: format!("cats{i}"),
                    },
                    kind: RelationKind::SharedInterest,
                    evidence: vec![RelationEvidence {
                        source: "bilibili".into(),
                        event_id: "e".into(),
                        quote: "我喜欢猫".into(),
                    }],
                    expires_at_ms: None,
                    admin_confirmed: true,
                    reason: "核实".into(),
                    request_key: format!("create{i}"),
                },
                now,
            )
            .await
            .unwrap();
        if i == 0 {
            sqlx::query("UPDATE memories SET expires_at_ms=$3 WHERE scope_id=$1 AND id=$2")
                .bind(&scope)
                .bind(memory)
                .bind(4_000_000_000_000i64)
                .execute(&pool)
                .await
                .unwrap();
            let before = s
                .query(&scope, &viewer, 1, 100, 3_999_999_999_999)
                .await
                .unwrap();
            assert_eq!(
                before.iter().find(|r| r.id == f.id).unwrap().expires_at_ms,
                Some(4_000_000_000_000)
            );
            let refs = [GraphReference {
                fact_id: f.id.clone(),
                version: 1,
            }];
            assert_eq!(
                s.validate_graph(&scope, &refs, 3_999_999_999_999)
                    .await
                    .unwrap()[0]
                    .expires_at_ms,
                Some(4_000_000_000_000)
            );
            assert_eq!(
                s.list_admin(&scope, &viewer, 100, 0)
                    .await
                    .unwrap()
                    .iter()
                    .find(|r| r.id == f.id)
                    .unwrap()
                    .expires_at_ms,
                None
            );
            assert!(
                s.query(&scope, &viewer, 1, 100, 4_000_000_000_000)
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert!(
                s.validate_graph(
                    &scope,
                    &[GraphReference {
                        fact_id: f.id.clone(),
                        version: 1
                    }],
                    4_000_000_000_000
                )
                .await
                .unwrap()
                .is_empty()
            );
        }
        // A leased old projection must not be acknowledged after source invalidation.
        while let Some(lease) = s.claim_outbox(&scope, now).await.unwrap() {
            if lease.id == f.id {
                break;
            }
            s.finish_outbox(&scope, &lease.id, &lease.token, now)
                .await
                .unwrap();
        }
        let mut tx = pool.begin().await.unwrap();
        sqlx::query(&format!(
            "UPDATE memories SET {operation},updated_at_ms=$3 WHERE scope_id=$1 AND id=$2"
        ))
        .bind(&scope)
        .bind(memory)
        .bind(now as i64 + 1)
        .execute(&mut *tx)
        .await
        .unwrap();
        let deleted: bool = sqlx::query_scalar(
            "SELECT deleted FROM relationship_facts WHERE scope_id=$1 AND id=$2",
        )
        .bind(&scope)
        .bind(uuid::Uuid::parse_str(&f.id).unwrap())
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert!(deleted, "operation {i}: {operation}");
        tx.commit().await.unwrap();
        assert!(
            s.validate_graph(
                &scope,
                &[GraphReference {
                    fact_id: f.id.clone(),
                    version: f.version
                }],
                now
            )
            .await
            .unwrap()
            .is_empty()
        );
        let facts = s.list_admin(&scope, &viewer, 100, 0).await.unwrap();
        let changed = facts.iter().find(|r| r.id == f.id).unwrap();
        assert!(changed.deleted);
        assert_eq!(changed.version, 2);
        let row: (String, Option<uuid::Uuid>) = sqlx::query_as(
            "SELECT state,token FROM relationship_outbox WHERE scope_id=$1 AND fact_id=$2",
        )
        .bind(&scope)
        .bind(uuid::Uuid::parse_str(&f.id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row, ("pending".into(), None));
    }
}
