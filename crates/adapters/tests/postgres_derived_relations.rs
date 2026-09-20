use meowlive_adapters::storage::postgres::PostgresViewerEventStore;
use meowlive_application::ports::{
    memory_store::{MemoryAdminRequest, MemoryStore},
    relationships::{RelationAction, RelationshipStore},
    viewers::ViewerEventStore,
};
use meowlive_domain::{event::*, memory::*};
fn event(id: &str, text: &str, t: u64) -> LiveEvent {
    LiveEvent {
        id: id.into(),
        source: "bilibili".into(),
        viewer: "观众".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "derived-test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "one".into(),
        }),
        occurred_at_ms: t,
        gift_metadata: None,
        kind: EventKind::Chat { text: text.into() },
    }
}
async fn accept(
    store: &PostgresViewerEventStore,
    scope: &str,
    id: &str,
    text: &str,
    key: &str,
    value: &str,
    kind: MemoryKind,
    t: u64,
) -> String {
    let viewer = store
        .accept_events(scope, "s", &[event(id, text, t)], t)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let job = store.claim_job(scope, t as i64).await.unwrap().unwrap();
    let c = MemoryCandidate {
        key: key.into(),
        value: value.into(),
        kind,
        evidence: vec![Evidence::from_source(&job.sources[0], text)],
        explicit: true,
        confidence: 1.0,
        valid_until_ms: None,
    };
    store
        .finish_job(scope, &job, &[c], t as i64 + 1)
        .await
        .unwrap();
    viewer
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn accepted_memories_derive_bounded_graph_facts_and_deletion_invalidates() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let scope = format!("derived-{}", uuid::Uuid::new_v4());
    let t = 1_800_000_000_000;
    let viewer = accept(
        &store,
        &scope,
        "p",
        "我喜欢猫",
        "preference",
        "猫",
        MemoryKind::Preference,
        t,
    )
    .await;
    accept(
        &store,
        &scope,
        "e",
        "我参加了音乐会",
        "experience",
        "音乐会",
        MemoryKind::Experience,
        t + 2,
    )
    .await;
    accept(
        &store,
        &scope,
        "c",
        "我认识小明",
        "third_party_claim",
        "小明",
        MemoryKind::ThirdPartyClaim,
        t + 4,
    )
    .await;
    let rows:Vec<(String,String,String,String)>=sqlx::query_as("SELECT target_kind,target_id,kind,confirmation FROM relationship_facts WHERE scope_id=$1 ORDER BY target_kind").bind(&scope).fetch_all(&pool).await.unwrap();
    assert!(rows.contains(&(
        "topic".into(),
        "猫".into(),
        "shared_interest".into(),
        "confirmed".into()
    )));
    assert!(rows.contains(&(
        "activity".into(),
        "音乐会".into(),
        "participated".into(),
        "confirmed".into()
    )));
    assert!(rows.contains(&(
        "unresolved".into(),
        "小明".into(),
        "mention".into(),
        "claimed".into()
    )));
    assert_eq!(rows.len(), 3);
    let record = store
        .list(&scope, &viewer, 20, t as i64 + 6)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.candidate.key == "preference")
        .unwrap();
    store
        .change(
            &scope,
            &record.id,
            1,
            RelationAction::Revoke,
            "admin revoked",
            "revoke",
            t + 7,
        )
        .await
        .unwrap();
    accept(
        &store,
        &scope,
        "p2",
        "我喜欢猫",
        "preference",
        "猫",
        MemoryKind::Preference,
        t + 8,
    )
    .await;
    let confirmation: String = sqlx::query_scalar(
        "SELECT confirmation FROM relationship_facts WHERE scope_id=$1 AND id=$2",
    )
    .bind(&scope)
    .bind(uuid::Uuid::parse_str(&record.id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(confirmation, "claimed");
    let record = store
        .list(&scope, &viewer, 20, t as i64 + 10)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.id == record.id)
        .unwrap();
    store
        .admin_delete(
            &scope,
            &MemoryAdminRequest {
                viewer_id: viewer,
                memory_id: record.id.clone(),
                expected_version: record.version,
                request_key: "delete".into(),
                reason: "remove".into(),
                actor: "admin".into(),
            },
            t as i64 + 11,
        )
        .await
        .unwrap();
    let deleted: bool =
        sqlx::query_scalar("SELECT deleted FROM relationship_facts WHERE scope_id=$1 AND id=$2")
            .bind(&scope)
            .bind(uuid::Uuid::parse_str(&record.id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(deleted);
}
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn unrelated_and_qualified_quotes_never_derive_confirmed_relationships() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let store = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("derived-unsafe-{}", uuid::Uuid::new_v4());
    let t = 1_800_000_000_000;
    accept(
        &store,
        &scope,
        "hypothesis",
        "如果我认识小明就好了",
        "third_party_claim",
        "小明",
        MemoryKind::ThirdPartyClaim,
        t,
    )
    .await;
    accept(
        &store,
        &scope,
        "stable",
        "我养猫",
        "stable_fact",
        "猫",
        MemoryKind::StableFact,
        t + 2,
    )
    .await;
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM relationship_facts WHERE scope_id=$1")
            .bind(&scope)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}
