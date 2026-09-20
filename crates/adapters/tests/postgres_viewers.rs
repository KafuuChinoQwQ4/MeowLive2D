use meowlive_adapters::storage::postgres::PostgresViewerEventStore;
use meowlive_application::ports::viewers::ViewerEventStore;
use meowlive_domain::event::{
    EventKind, GiftMetadata, LiveEvent, ViewerIdentity, ViewerIdentityKind,
};
use sqlx::PgPool;

async fn cleanup_scope(database_url: &str, scope_id: &str) {
    let pool = PgPool::connect(database_url)
        .await
        .expect("test cleanup must connect to dedicated PostgreSQL");
    for table in [
        "viewer_events",
        "viewer_aliases",
        "viewer_identities",
        "viewers",
        "live_sessions",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE scope_id = $1"))
            .bind(scope_id)
            .execute(&pool)
            .await
            .expect("test cleanup must remove its own rows");
    }
}

fn chat(
    id: &str,
    viewer: &str,
    occurred_at_ms: u64,
    viewer_identity: Option<ViewerIdentity>,
) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "simulator".into(),
        viewer: viewer.into(),
        viewer_identity,
        occurred_at_ms,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: "hello".into(),
        },
    }
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn duplicate_anonymous_event_is_persisted_once_without_a_viewer_profile() {
    let database_url = std::env::var("MEOWLIVE_TEST_DATABASE_URL")
        .expect("MEOWLIVE_TEST_DATABASE_URL must point at the dedicated MeowLive2D PostgreSQL");
    let store = PostgresViewerEventStore::connect(&database_url)
        .await
        .expect("dedicated PostgreSQL must accept migrations");
    let scope_id = format!("postgres-viewers-{}", uuid::Uuid::new_v4());
    let event = chat("anonymous-1", "display-only", 1_700_000_000_000, None);

    let first = store
        .accept_events(&scope_id, "session-1", &[event.clone()], 1_700_000_000_001)
        .await
        .expect("first event must persist");
    let repeated = store
        .accept_events(&scope_id, "session-1", &[event], 1_700_000_000_002)
        .await
        .expect("duplicate lookup must succeed");

    assert_eq!(first.len(), 1);
    assert!(!first[0].duplicate);
    assert_eq!(first[0].viewer_id, None);
    assert_eq!(repeated.len(), 1);
    assert!(repeated[0].duplicate);
    assert_eq!(repeated[0].viewer_id, None);

    let events = store
        .list_events(&scope_id, 10, 0)
        .await
        .expect("stored event must be queryable");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, "anonymous-1");
    assert_eq!(events[0].viewer_id, None);
    assert_eq!(events[0].viewer, "display-only");
    assert_eq!(events[0].occurred_at_ms, 1_700_000_000_000);
    assert_eq!(events[0].received_at_ms, 1_700_000_000_001);
    cleanup_scope(&database_url, &scope_id).await;
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn stable_identity_keeps_latest_alias_and_raw_gift_metadata() {
    let database_url = std::env::var("MEOWLIVE_TEST_DATABASE_URL")
        .expect("MEOWLIVE_TEST_DATABASE_URL must point at the dedicated MeowLive2D PostgreSQL");
    let store = PostgresViewerEventStore::connect(&database_url)
        .await
        .expect("dedicated PostgreSQL must accept migrations");
    let scope_id = format!("postgres-viewers-{}", uuid::Uuid::new_v4());
    let identity = ViewerIdentity {
        namespace: "simulator:room-1".into(),
        kind: ViewerIdentityKind::OpenId,
        external_id: "open-1".into(),
    };
    let newest = chat(
        "identity-newest",
        "Current Alias",
        1_700_000_000_200,
        Some(identity.clone()),
    );
    let older = chat(
        "identity-older",
        "Former Alias",
        1_700_000_000_100,
        Some(identity),
    );
    let gift = LiveEvent {
        id: "identity-gift".into(),
        source: "simulator".into(),
        viewer: "Current Alias".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "simulator:room-1".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "open-1".into(),
        }),
        occurred_at_ms: 1_700_000_000_300,
        gift_metadata: Some(GiftMetadata {
            price: Some(300),
            paid: None,
            medal_level: Some(12),
            guard_level: None,
        }),
        kind: EventKind::Gift {
            name: "Cat".into(),
            count: 2,
        },
    };

    let outcomes = store
        .accept_events(
            &scope_id,
            "session-identity",
            &[newest, older, gift.clone(), gift],
            1_700_000_000_400,
        )
        .await
        .expect("identity batch must persist atomically");
    let viewer_id = outcomes[0]
        .viewer_id
        .clone()
        .expect("stable identity must create a viewer");
    assert!(!outcomes[0].duplicate);
    assert!(!outcomes[1].duplicate);
    assert!(!outcomes[2].duplicate);
    assert!(outcomes[3].duplicate);
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome.viewer_id.as_deref() == Some(&viewer_id))
    );

    let viewers = store
        .list_viewers(&scope_id, 10, 0)
        .await
        .expect("viewer must be queryable");
    assert_eq!(viewers.len(), 1);
    assert_eq!(viewers[0].viewer_id, viewer_id);
    assert_eq!(viewers[0].current_alias.as_deref(), Some("Current Alias"));
    assert_eq!(viewers[0].alias_observed_at_ms, Some(1_700_000_000_300));
    assert_eq!(
        viewers[0]
            .aliases
            .iter()
            .map(|alias| alias.alias.as_str())
            .collect::<Vec<_>>(),
        ["Current Alias", "Former Alias"]
    );
    assert_eq!(viewers[0].identities.len(), 1);
    assert_eq!(viewers[0].identities[0].external_id, "open-1");

    let events = store
        .list_events(&scope_id, 10, 0)
        .await
        .expect("stored gift must be queryable");
    let persisted_gift = events
        .iter()
        .find(|event| event.event_id == "identity-gift")
        .expect("gift event must be listed");
    assert_eq!(
        persisted_gift.viewer_id.as_deref(),
        Some(viewer_id.as_str())
    );
    assert_eq!(
        persisted_gift.gift_metadata,
        Some(GiftMetadata {
            price: Some(300),
            paid: None,
            medal_level: Some(12),
            guard_level: None,
        })
    );
    cleanup_scope(&database_url, &scope_id).await;
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn concurrent_batches_share_one_stable_viewer_and_reject_oversized_batches() {
    let database_url = std::env::var("MEOWLIVE_TEST_DATABASE_URL")
        .expect("MEOWLIVE_TEST_DATABASE_URL must point at the dedicated MeowLive2D PostgreSQL");
    let store = PostgresViewerEventStore::connect(&database_url)
        .await
        .expect("dedicated PostgreSQL must accept migrations");
    let scope_id = format!("postgres-viewers-{}", uuid::Uuid::new_v4());
    let first_identity = ViewerIdentity {
        namespace: "simulator:room-concurrent".into(),
        kind: ViewerIdentityKind::OpenId,
        external_id: "open-concurrent-a".into(),
    };
    let second_identity = ViewerIdentity {
        namespace: "simulator:room-concurrent".into(),
        kind: ViewerIdentityKind::OpenId,
        external_id: "open-concurrent-b".into(),
    };
    let first_store = store.clone();
    let first_scope = scope_id.clone();
    let first_a = first_identity.clone();
    let first_b = second_identity.clone();
    let first = tokio::spawn(async move {
        first_store
            .accept_events(
                &first_scope,
                "session-concurrent-a",
                &[
                    chat("concurrent-a", "Alias A", 1_700_000_010_000, Some(first_a)),
                    chat("concurrent-b", "Alias B", 1_700_000_010_100, Some(first_b)),
                ],
                1_700_000_010_100,
            )
            .await
    });
    let second_store = store.clone();
    let second_scope = scope_id.clone();
    let second_a = first_identity;
    let second_b = second_identity;
    let second = tokio::spawn(async move {
        second_store
            .accept_events(
                &second_scope,
                "session-concurrent-b",
                &[
                    chat("concurrent-d", "Alias B", 1_700_000_010_300, Some(second_b)),
                    chat("concurrent-c", "Alias A", 1_700_000_010_200, Some(second_a)),
                ],
                1_700_000_010_300,
            )
            .await
    });
    let first = first
        .await
        .expect("first task must join")
        .expect("first batch must persist");
    let second = second
        .await
        .expect("second task must join")
        .expect("second batch must persist");
    assert_eq!(first[0].viewer_id, second[1].viewer_id);
    assert_eq!(first[1].viewer_id, second[0].viewer_id);
    assert_eq!(
        store
            .list_viewers(&scope_id, 10, 0)
            .await
            .expect("concurrent viewer must be queryable")
            .len(),
        2
    );

    let too_many = (0..101)
        .map(|index| {
            chat(
                &format!("oversized-{index}"),
                "display-only",
                1_700_000_020_000 + index,
                None,
            )
        })
        .collect::<Vec<_>>();
    let error = store
        .accept_events(&scope_id, "session-oversized", &too_many, 1_700_000_020_200)
        .await
        .expect_err("port must bound batches before database work");
    assert!(error.message.contains("between 1 and 100"));
    cleanup_scope(&database_url, &scope_id).await;
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn identity_scope_isolation_nickname_collisions_and_reopened_duplicates_are_preserved() {
    let database_url = std::env::var("MEOWLIVE_TEST_DATABASE_URL")
        .expect("MEOWLIVE_TEST_DATABASE_URL must point at the dedicated MeowLive2D PostgreSQL");
    let store = PostgresViewerEventStore::connect(&database_url)
        .await
        .expect("dedicated PostgreSQL must accept migrations");
    let scope_id = format!("postgres-viewers-{}", uuid::Uuid::new_v4());
    let other_scope_id = format!("postgres-viewers-{}", uuid::Uuid::new_v4());
    let one = ViewerIdentity {
        namespace: "simulator:room-collision".into(),
        kind: ViewerIdentityKind::OpenId,
        external_id: "open-one".into(),
    };
    let two = ViewerIdentity {
        namespace: "simulator:room-collision".into(),
        kind: ViewerIdentityKind::OpenId,
        external_id: "open-two".into(),
    };
    let first = chat(
        "collision-one",
        "Shared Alias",
        1_700_000_030_000,
        Some(one),
    );
    let second = chat(
        "collision-two",
        "Shared Alias",
        1_700_000_030_100,
        Some(two),
    );
    let outcomes = store
        .accept_events(
            &scope_id,
            "session-collision",
            &[first.clone(), second],
            1_700_000_030_200,
        )
        .await
        .expect("same nickname with distinct stable IDs must persist");
    assert_ne!(outcomes[0].viewer_id, outcomes[1].viewer_id);
    assert_eq!(
        store
            .list_viewers(&scope_id, 10, 0)
            .await
            .expect("collision viewers must be queryable")
            .len(),
        2
    );
    let reopened = PostgresViewerEventStore::connect(&database_url)
        .await
        .expect("reopened store must migrate and connect");
    // Even identical external IDs, namespaces and event IDs cannot merge platforms.
    let mut platform_event = first.clone();
    platform_event.source = "bilibili".into();
    platform_event.viewer = "Platform Alias".into();
    let platform = reopened
        .accept_events(
            &scope_id,
            "session-platform",
            &[platform_event],
            1_700_000_030_300,
        )
        .await
        .expect("real platform identity must be isolated from simulation");
    assert!(!platform[0].duplicate);
    assert_ne!(platform[0].viewer_id, outcomes[0].viewer_id);
    let profiles = reopened.list_viewers(&scope_id, 10, 0).await.unwrap();
    let simulated = profiles
        .iter()
        .find(|v| Some(&v.viewer_id) == outcomes[0].viewer_id.as_ref())
        .unwrap();
    assert_eq!(simulated.current_alias.as_deref(), Some("Shared Alias"));
    assert_eq!(simulated.aliases.len(), 1);
    let real = profiles
        .iter()
        .find(|v| Some(&v.viewer_id) == platform[0].viewer_id.as_ref())
        .unwrap();
    assert_eq!(real.current_alias.as_deref(), Some("Platform Alias"));
    assert_eq!(real.aliases.len(), 1);
    let duplicate = reopened
        .accept_events(
            &scope_id,
            "session-collision-reopened",
            &[first],
            1_700_000_030_300,
        )
        .await
        .expect("reopened store must detect persisted duplicate");
    assert!(duplicate[0].duplicate);
    assert_eq!(duplicate[0].viewer_id, outcomes[0].viewer_id);

    let isolated = reopened
        .accept_events(
            &other_scope_id,
            "session-isolated",
            &[chat(
                "collision-one",
                "Shared Alias",
                1_700_000_030_400,
                Some(ViewerIdentity {
                    namespace: "simulator:room-collision".into(),
                    kind: ViewerIdentityKind::OpenId,
                    external_id: "open-one".into(),
                }),
            )],
            1_700_000_030_500,
        )
        .await
        .expect("same external ID is isolated by scope");
    assert_ne!(isolated[0].viewer_id, outcomes[0].viewer_id);

    let rejected_scope_id = format!("postgres-viewers-{}", uuid::Uuid::new_v4());
    let invalid = chat("invalid-batch", " ", 1_700_000_030_600, None);
    let error = reopened
        .accept_events(
            &rejected_scope_id,
            "session-invalid",
            &[
                chat("valid-batch", "valid", 1_700_000_030_600, None),
                invalid,
            ],
            1_700_000_030_700,
        )
        .await
        .expect_err("an invalid event must reject the complete batch");
    assert!(error.message.contains("viewer"));
    assert!(
        reopened
            .list_events(&rejected_scope_id, 10, 0)
            .await
            .expect("rejected scope must be queryable")
            .is_empty()
    );
    cleanup_scope(&database_url, &scope_id).await;
    cleanup_scope(&database_url, &other_scope_id).await;
    cleanup_scope(&database_url, &rejected_scope_id).await;
}
