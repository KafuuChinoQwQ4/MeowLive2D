use meowlive_adapters::storage::postgres::PostgresViewerEventStore;
use meowlive_application::ports::{companionship::*, viewers::ViewerEventStore};
use meowlive_domain::event::*;
#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn presence_completed_gifts_and_admin_are_idempotent() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let s = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("companionship-{}", uuid::Uuid::new_v4());
    let mut e = LiveEvent {
        id: "chat1".into(),
        source: "bilibili".into(),
        viewer: "猫".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "a".into(),
        }),
        occurred_at_ms: 1_700_000_000_000,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: "今天很开心".into(),
        },
    };
    let v = s
        .accept_events(&scope, "one", &[e.clone()], e.occurred_at_ms)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    assert_eq!(
        s.detail(&scope, &v, 20)
            .await
            .unwrap()
            .unwrap()
            .familiarity_milli,
        1000
    );
    s.register_reply(&scope, "speech1", &[e.clone()], e.occurred_at_ms)
        .await
        .unwrap();
    assert_eq!(
        s.detail(&scope, &v, 20)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        0
    );
    s.complete_reply(&scope, "speech1", e.occurred_at_ms)
        .await
        .unwrap();
    s.complete_reply(&scope, "speech1", e.occurred_at_ms)
        .await
        .unwrap();
    e.id = "chat2".into();
    s.accept_events(&scope, "two", &[e.clone()], e.occurred_at_ms)
        .await
        .unwrap();
    s.register_reply(&scope, "speech2", &[e.clone()], e.occurred_at_ms)
        .await
        .unwrap();
    s.complete_reply(&scope, "speech2", e.occurred_at_ms)
        .await
        .unwrap();
    assert_eq!(
        s.detail(&scope, &v, 20)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        200
    );
    e.id = "gift".into();
    e.kind = EventKind::Gift {
        name: "礼物".into(),
        count: 20,
    };
    e.gift_metadata = Some(GiftMetadata {
        price: Some(999),
        paid: Some(true),
        medal_level: Some(3),
        guard_level: None,
    });
    s.accept_events(&scope, "two", &[e.clone()], e.occurred_at_ms)
        .await
        .unwrap();
    let d = s.detail(&scope, &v, 20).await.unwrap().unwrap();
    assert_eq!(d.gifts[0].value_cents, None);
    assert_eq!(d.affinity_milli, 200);
    let c = GiftConfirmation {
        request_key: "confirm".into(),
        reason: "收据核实".into(),
        source: e.source.clone(),
        event_id: e.id.clone(),
        value_cents: 100,
        value_kind: "confirmed_paid_value".into(),
    };
    s.confirm_gift(&scope, &v, &c, e.occurred_at_ms)
        .await
        .unwrap();
    s.confirm_gift(&scope, &v, &c, e.occurred_at_ms)
        .await
        .unwrap();
    assert_eq!(
        s.detail(&scope, &v, 20)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        269
    );
    let a = AffinityAdjustment {
        request_key: "adjust".into(),
        reason: "人工核对".into(),
        delta_milli: 100000,
    };
    let id = s.adjust(&scope, &v, &a, e.occurred_at_ms).await.unwrap();
    assert_eq!(
        s.adjust(&scope, &v, &a, e.occurred_at_ms).await.unwrap(),
        id
    );
    assert_eq!(
        s.detail(&scope, &v, 20)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        100000
    );
    s.reverse(
        &scope,
        &v,
        &AffinityReversal {
            request_key: "reverse".into(),
            reason: "撤销".into(),
            ledger_id: id,
        },
        e.occurred_at_ms,
    )
    .await
    .unwrap();
    assert_eq!(
        s.detail(&scope, &v, 20)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        269
    );
    assert!(s.detail("other", &v, 20).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn concurrent_caps_late_events_free_gifts_and_restart() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let s = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("companionship-caps-{}", uuid::Uuid::new_v4());
    let now = 1_700_000_000_000;
    let base = LiveEvent {
        id: "first".into(),
        source: "bilibili".into(),
        viewer: "猫".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "a".into(),
        }),
        occurred_at_ms: now,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: "first".into(),
        },
    };
    let v = s
        .accept_events(&scope, "one", &[base.clone()], now)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    let mut handles = Vec::new();
    for i in 0..15 {
        let mut e = base.clone();
        e.id = format!("chat{i}");
        e.kind = EventKind::Chat {
            text: format!("unique body {i}"),
        };
        let st = s.clone();
        let sc = scope.clone();
        handles.push(tokio::spawn(async move {
            st.accept_events(&sc, "one", &[e.clone()], now)
                .await
                .unwrap();
            let speech = format!("s{i}");
            st.register_reply(&sc, &speech, &[e], now).await.unwrap();
            st.complete_reply(&sc, &speech, now).await.unwrap();
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert_eq!(
        s.detail(&scope, &v, 100)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        2000
    );
    let mut late = base.clone();
    late.id = "late".into();
    late.occurred_at_ms -= 86_400_000;
    s.accept_events(&scope, "old", &[late.clone()], now)
        .await
        .unwrap();
    s.register_reply(&scope, "late", &[late], now)
        .await
        .unwrap();
    s.complete_reply(&scope, "late", now).await.unwrap();
    let d = s.detail(&scope, &v, 100).await.unwrap().unwrap();
    assert_eq!(d.affinity_milli, 2200);
    assert_eq!(d.familiarity_milli, 2000);
    assert_eq!(d.last_seen_at_ms, now);
    let mut gift = base.clone();
    gift.id = "free".into();
    gift.kind = EventKind::Gift {
        name: "free".into(),
        count: 2,
    };
    gift.gift_metadata = Some(GiftMetadata {
        paid: Some(false),
        ..Default::default()
    });
    s.accept_events(&scope, "one", &[gift.clone()], now)
        .await
        .unwrap();
    let c = GiftConfirmation {
        request_key: "free-confirm".into(),
        reason: "test receipt".into(),
        source: gift.source.clone(),
        event_id: gift.id.clone(),
        value_cents: 100,
        value_kind: "confirmed_paid_value".into(),
    };
    assert!(s.confirm_gift(&scope, &v, &c, now).await.is_err());
    let a = AffinityAdjustment {
        request_key: "one".into(),
        reason: "test".into(),
        delta_milli: 100000,
    };
    s.adjust(&scope, &v, &a, now).await.unwrap();
    let mut changed = a.clone();
    changed.delta_milli = 1;
    assert!(s.adjust(&scope, &v, &changed, now).await.is_err());
    gift.id = "paid".into();
    gift.gift_metadata = None;
    s.accept_events(&scope, "one", &[gift.clone()], now)
        .await
        .unwrap();
    let mut c = c;
    c.request_key = "paid-confirm".into();
    c.event_id = gift.id;
    s.confirm_gift(&scope, &v, &c, now).await.unwrap();
    let d = s.detail(&scope, &v, 100).await.unwrap().unwrap();
    let ledger = d.ledger.iter().find(|l| l.kind == "gift").unwrap();
    assert_eq!(ledger.computed_delta_milli, 69);
    assert_eq!(ledger.applied_delta_milli, 0);
    let restarted = PostgresViewerEventStore::connect(&url).await.unwrap();
    restarted.complete_reply(&scope, "late", now).await.unwrap();
    assert_eq!(
        restarted
            .detail(&scope, &v, 100)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        100000
    );
    let mut anonymous = base.clone();
    anonymous.id = "anon".into();
    anonymous.viewer_identity = None;
    assert!(
        s.accept_events(&scope, "one", &[anonymous], now)
            .await
            .unwrap()[0]
            .viewer_id
            .is_none()
    );
    let mut sim = base;
    sim.source = "simulator".into();
    let sv = s.accept_events(&scope, "one", &[sim], now).await.unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    assert!(s.detail(&scope, &sv, 100).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires dedicated PostgreSQL and MEOWLIVE_TEST_DATABASE_URL"]
async fn reply_identity_batch_award_and_reversal_keep_deduplication() {
    let url = std::env::var("MEOWLIVE_TEST_DATABASE_URL").unwrap();
    let s = PostgresViewerEventStore::connect(&url).await.unwrap();
    let scope = format!("companionship-review-{}", uuid::Uuid::new_v4());
    let now = 1_700_000_000_000;
    let mut e = LiveEvent {
        id: "c".into(),
        source: "a:b".into(),
        viewer: "猫".into(),
        viewer_identity: Some(ViewerIdentity {
            namespace: "test".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "a".into(),
        }),
        occurred_at_ms: now,
        gift_metadata: None,
        kind: EventKind::Chat {
            text: "first".into(),
        },
    };
    let mut e2 = e.clone();
    e2.id = "second".into();
    e2.kind = EventKind::Chat {
        text: "second".into(),
    };
    let events = vec![e.clone(), e2.clone()];
    let v = s.accept_events(&scope, "one", &events, now).await.unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    s.register_reply(&scope, "speech", &events, now)
        .await
        .unwrap();
    assert!(
        s.register_reply(&scope, "speech", &[e.clone()], now)
            .await
            .is_err()
    );
    s.register_reply(&scope, "speech", &[e2.clone(), e.clone()], now)
        .await
        .unwrap();
    s.complete_reply(&scope, "speech", now).await.unwrap();
    let d = s.detail(&scope, &v, 100).await.unwrap().unwrap();
    assert_eq!(d.affinity_milli, 200);
    e2.id = "missing".into();
    assert!(
        s.register_reply(&scope, "missing", &[e2], now)
            .await
            .is_err()
    );
    s.register_reply(&scope, "proactive", &[], now)
        .await
        .unwrap();
    s.complete_reply(&scope, "proactive", now).await.unwrap();
    let l = d
        .ledger
        .iter()
        .find(|l| l.kind == "exchange" && l.applied_delta_milli == 200)
        .unwrap();
    let reversal = AffinityReversal {
        request_key: "reverse-exchange".into(),
        reason: "错误交流".into(),
        ledger_id: l.ledger_id.clone(),
    };
    s.reverse(&scope, &v, &reversal, now).await.unwrap();
    s.register_reply(&scope, "again", &events, now)
        .await
        .unwrap();
    s.complete_reply(&scope, "again", now).await.unwrap();
    assert_eq!(
        s.detail(&scope, &v, 100)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        0
    );
    // Both source/event pairs previously collided under colon concatenation.
    e.source = "a".into();
    e.id = "b:c".into();
    let v2 = s
        .accept_events(&scope, "one", &[e.clone()], now)
        .await
        .unwrap()[0]
        .viewer_id
        .clone()
        .unwrap();
    s.register_reply(&scope, "collision", &[e], now)
        .await
        .unwrap();
    s.complete_reply(&scope, "collision", now).await.unwrap();
    assert_eq!(
        s.detail(&scope, &v2, 100)
            .await
            .unwrap()
            .unwrap()
            .affinity_milli,
        200
    );
}
