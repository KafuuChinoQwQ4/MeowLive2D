use meowlive_domain::memory::*;
fn source(id: &str, day: i64, text: &str) -> MemorySource {
    MemorySource {
        source: "test".into(),
        viewer_id: "v".into(),
        event_id: id.into(),
        text: text.into(),
        occurred_at_ms: day * DAY_MS,
        day,
    }
}
fn candidate(s: &MemorySource) -> MemoryCandidate {
    MemoryCandidate {
        key: "preference".into(),
        value: "猫".into(),
        kind: MemoryKind::Preference,
        evidence: vec![Evidence::from_source(s, "我喜欢猫")],
        explicit: true,
        confidence: 0.99,
        valid_until_ms: None,
    }
}
#[test]
fn explicit_preference_can_be_long_term_but_forged_value_cannot() {
    let s = source("a", 1, "我喜欢猫");
    let mut c = candidate(&s);
    assert_eq!(
        assess(&c, &[s.clone()], DAY_MS).unwrap().status,
        MemoryStatus::LongTerm
    );
    c.value = "狗".into();
    assert!(assess(&c, &[s], DAY_MS).is_err());
}
#[test]
fn duplicate_evidence_and_model_flags_do_not_promote() {
    let s = source("a", 1, "也许猫不错");
    let mut c = candidate(&s);
    c.evidence = vec![Evidence::from_source(&s, &s.text); 2];
    assert_eq!(
        assess(&c, &[s], DAY_MS).unwrap().status,
        MemoryStatus::Candidate
    );
}
#[test]
fn expiry_and_half_life_are_fixed() {
    let s = source("a", 1, "我今天很累");
    let mut c = candidate(&s);
    c.key = "temporary_state".into();
    c.value = "累".into();
    c.kind = MemoryKind::TemporaryState;
    c.evidence = vec![Evidence::from_source(&s, &s.text)];
    let a = assess(&c, &[s.clone()], DAY_MS).unwrap();
    assert_eq!(a.expires_at_ms, Some(2 * DAY_MS));
    assert_eq!(
        assess(&c, &[s], 2 * DAY_MS).unwrap().status,
        MemoryStatus::Expired
    );
    assert!((short_term_weight(7 * DAY_MS) - 0.5).abs() < 1e-9);
}
#[test]
fn ordinary_stable_statement_needs_distinct_days() {
    let a = source("a", 1, "我养猫");
    let b = source("b", 2, "我养猫");
    let mut c = candidate(&a);
    c.key = "stable_fact".into();
    c.kind = MemoryKind::StableFact;
    c.evidence = vec![Evidence::from_source(&a, &a.text)];
    assert_eq!(
        assess(&c, &[a.clone()], 2 * DAY_MS).unwrap().status,
        MemoryStatus::Candidate
    );
    c.evidence.push(Evidence::from_source(&b, &b.text));
    assert_eq!(
        assess(&c, &[a, b], 2 * DAY_MS).unwrap().status,
        MemoryStatus::LongTerm
    );
}
#[test]
fn ordinary_experience_expires_without_retrieval_refresh() {
    let s = source("a", 1, "我昨天去了公园");
    let mut c = candidate(&s);
    c.kind = MemoryKind::Experience;
    c.key = "experience".into();
    c.value = "公园".into();
    c.evidence = vec![Evidence::from_source(&s, &s.text)];
    assert_eq!(
        assess(&c, &[s.clone()], DAY_MS).unwrap().expires_at_ms,
        Some(31 * DAY_MS)
    );
    assert_eq!(
        assess(&c, &[s.clone()], 10 * DAY_MS).unwrap().expires_at_ms,
        Some(31 * DAY_MS)
    );
    assert_eq!(
        assess(&c, &[s], 31 * DAY_MS).unwrap().status,
        MemoryStatus::Expired
    );
}
#[test]
fn long_term_has_no_automatic_expiry_but_respects_explicit_deadline() {
    let s = source("a", 1, "我喜欢猫");
    let mut c = candidate(&s);
    assert_eq!(
        assess(&c, &[s.clone()], 1000 * DAY_MS)
            .unwrap()
            .expires_at_ms,
        None
    );
    c.valid_until_ms = Some(2 * DAY_MS);
    assert_eq!(
        assess(&c, &[s], 2 * DAY_MS).unwrap().status,
        MemoryStatus::Expired
    );
}
#[test]
fn forged_time_and_cross_viewer_sources_are_rejected() {
    let s = source("a", 1, "我喜欢猫");
    let mut c = candidate(&s);
    c.evidence[0].day = 2;
    assert!(assess(&c, &[s.clone()], DAY_MS).is_err());
    c = candidate(&s);
    let mut other = source("b", 1, "我喜欢猫");
    other.viewer_id = "other".into();
    assert!(assess(&c, &[s, other], DAY_MS).is_err());
}
#[test]
fn third_party_and_sensitive_claims_never_auto_confirm() {
    for kind in [MemoryKind::ThirdPartyClaim, MemoryKind::SensitiveInference] {
        let a = source("a", 1, "我认识猫");
        let b = source("b", 2, "我认识猫");
        let mut c = candidate(&a);
        c.kind = kind;
        c.key = if kind == MemoryKind::ThirdPartyClaim {
            "third_party_claim"
        } else {
            "sensitive_inference"
        }
        .into();
        c.evidence = vec![
            Evidence::from_source(&a, &a.text),
            Evidence::from_source(&b, &b.text),
        ];
        assert_eq!(
            assess(&c, &[a, b], 2 * DAY_MS).unwrap().status,
            MemoryStatus::Candidate
        );
    }
}
#[test]
fn ambiguous_preference_needs_two_days_and_statement_shape() {
    let a = source("a", 1, "我觉得猫不错");
    let b = source("b", 2, "我觉得猫不错");
    let mut c = candidate(&a);
    c.evidence = vec![Evidence::from_source(&a, &a.text)];
    assert_eq!(
        assess(&c, &[a.clone()], 2 * DAY_MS).unwrap().status,
        MemoryStatus::Candidate
    );
    c.evidence.push(Evidence::from_source(&b, &b.text));
    assert_eq!(
        assess(&c, &[a, b], 2 * DAY_MS).unwrap().status,
        MemoryStatus::LongTerm
    );
}
#[test]
fn stable_fact_cannot_launder_third_party_or_hypothetical_claim() {
    for text in ["我听说他养猫", "我如果养猫就好了", "我患有猫过敏"] {
        let a = source("a", 1, text);
        let b = source("b", 2, text);
        let mut c = candidate(&a);
        c.key = "stable_fact".into();
        c.kind = MemoryKind::StableFact;
        c.evidence = vec![
            Evidence::from_source(&a, text),
            Evidence::from_source(&b, text),
        ];
        assert_eq!(
            assess(&c, &[a, b], 2 * DAY_MS).unwrap().status,
            MemoryStatus::Candidate
        );
    }
}
#[test]
fn model_cannot_swallow_qualifier_into_preference_value() {
    let s = source("a", 1, "我喜欢猫，但只是开玩笑");
    let mut c = candidate(&s);
    c.value = "猫，但只是开玩笑".into();
    c.evidence = vec![Evidence::from_source(&s, &s.text)];
    assert_eq!(
        assess(&c, &[s], DAY_MS).unwrap().status,
        MemoryStatus::Candidate
    );
}
