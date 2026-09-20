use meowlive_adapters::graph::Neo4jRelationshipGraph;
use meowlive_application::ports::relationships::*;
#[tokio::test]
#[ignore = "requires dedicated Neo4j and MEOWLIVE_TEST_NEO4J_PASSWORD"]
async fn projection_version_gate_and_scoped_neighbors() {
    let g = Neo4jRelationshipGraph::connect(
        "http://127.0.0.1:17474",
        "neo4j",
        &std::env::var("MEOWLIVE_TEST_NEO4J_PASSWORD").unwrap(),
    )
    .await
    .unwrap();
    let scope = format!("graph-test-{}", uuid::Uuid::new_v4());
    let viewer = uuid::Uuid::new_v4().to_string();
    let mut p = GraphProjection {
        scope: scope.clone(),
        fact: RelationshipFact {
            id: uuid::Uuid::new_v4().to_string(),
            version: 1,
            source: RelationEntity {
                kind: EntityKind::Viewer,
                id: viewer.clone(),
            },
            target: RelationEntity {
                kind: EntityKind::Topic,
                id: "cats".into(),
            },
            kind: RelationKind::SharedInterest,
            confirmation: RelationConfirmation::Confirmed,
            evidence: vec![],
            expires_at_ms: None,
            deleted: false,
        },
    };
    g.project(&p).await.unwrap();
    assert_eq!(g.neighbors(&scope, &viewer, 1, 10).await.unwrap().len(), 1);
    assert!(
        g.neighbors("other", &viewer, 1, 10)
            .await
            .unwrap()
            .is_empty()
    );
    let old = p.clone();
    p.fact.version = 2;
    p.fact.deleted = true;
    g.project(&p).await.unwrap();
    g.project(&old).await.unwrap();
    assert!(
        g.neighbors(&scope, &viewer, 2, 10)
            .await
            .unwrap()
            .is_empty()
    );
    let (a, b) = tokio::join!(g.project(&old), g.project(&p));
    a.unwrap();
    b.unwrap();
    assert!(
        g.neighbors(&scope, &viewer, 1, 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(g.neighbors(&scope, &viewer, 3, 10).await.is_err());
}
