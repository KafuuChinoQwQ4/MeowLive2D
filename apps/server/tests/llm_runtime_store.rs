use meowlive_protocol::llm_runtime::{AgentRuntimeSettingsRequest, LlmPrice, LlmTokenUsage};
use meowlive_server::llm_runtime::{RuntimeStore, UsageQuery, estimate_cost};
use serde_json::json;
use std::path::PathBuf;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("meowlive-runtime-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn price() -> LlmPrice {
    LlmPrice {
        provider: "fixture".into(),
        base_url: "https://example.test/v1".into(),
        model: "chat".into(),
        input_usd_per_million: 2.,
        output_usd_per_million: 10.,
        cache_read_usd_per_million: 0.5,
        cache_write_usd_per_million: 3.,
    }
}

#[test]
fn cached_input_and_reasoning_are_subsets_not_extra_billable_tokens() {
    let usage = LlmTokenUsage {
        input_tokens: Some(1000),
        output_tokens: Some(200),
        cache_read_tokens: Some(400),
        cache_write_tokens: Some(100),
        reasoning_tokens: Some(150),
    };
    // 500 * 2 + 400 * 0.5 + 100 * 3 + 200 * 10 = 3500 micro USD.
    assert_eq!(estimate_cost(&usage, Some(&price())), Some(3500));
    let mut invalid = usage;
    invalid.cache_read_tokens = Some(1001);
    assert_eq!(estimate_cost(&invalid, Some(&price())), None);
}

#[test]
fn missing_usage_or_necessary_price_details_never_becomes_zero_cost() {
    let usage = LlmTokenUsage {
        input_tokens: Some(10),
        output_tokens: Some(2),
        ..Default::default()
    };
    assert_eq!(estimate_cost(&usage, None), None);
    assert_eq!(
        estimate_cost(&LlmTokenUsage::default(), Some(&price())),
        None
    );
    assert_eq!(estimate_cost(&usage, Some(&price())), None);
    let uniform = LlmPrice {
        cache_read_usd_per_million: 2.,
        cache_write_usd_per_million: 2.,
        ..price()
    };
    assert_eq!(estimate_cost(&usage, Some(&uniform)), Some(40));
}

#[test]
fn settings_are_durable_and_keys_are_kept_only_for_same_destination() {
    let directory = Directory::new();
    let store = RuntimeStore::open(&directory.0).unwrap();
    let mut settings = store.settings();
    settings.streaming = false;
    store
        .save(AgentRuntimeSettingsRequest {
            settings: settings.clone(),
            search_api_key: Some("test-search-key".into()),
            clear_search_api_key: false,
        })
        .unwrap();
    let reopened = RuntimeStore::open(&directory.0).unwrap();
    assert!(!reopened.settings().streaming);
    assert_eq!(reopened.search_key().as_deref(), Some("test-search-key"));
    assert!(
        !serde_json::to_string(&reopened.snapshot())
            .unwrap()
            .contains("test-search-key")
    );
    reopened
        .save(AgentRuntimeSettingsRequest {
            settings: settings.clone(),
            search_api_key: None,
            clear_search_api_key: false,
        })
        .unwrap();
    assert!(reopened.snapshot().search_key_configured);
    settings.search_endpoint = "https://other.example.test/search".into();
    reopened
        .save(AgentRuntimeSettingsRequest {
            settings: settings.clone(),
            search_api_key: None,
            clear_search_api_key: false,
        })
        .unwrap();
    assert!(!reopened.snapshot().search_key_configured);
    reopened
        .save(AgentRuntimeSettingsRequest {
            settings: settings.clone(),
            search_api_key: Some("replacement-key".into()),
            clear_search_api_key: false,
        })
        .unwrap();
    reopened
        .save(AgentRuntimeSettingsRequest {
            settings,
            search_api_key: None,
            clear_search_api_key: true,
        })
        .unwrap();
    assert!(
        RuntimeStore::open(&directory.0)
            .unwrap()
            .search_key()
            .is_none()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(directory.0.join("settings.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn invalid_search_addresses_prices_and_key_conflicts_are_rejected() {
    let store = RuntimeStore::memory();
    for endpoint in [
        "http://public.example/search",
        "https://user:password@example.test",
        "https://example.test/?key=x",
        "https://example.test/#fragment",
        "https://example.test\\evil",
    ] {
        let mut settings = store.settings();
        settings.search_endpoint = endpoint.into();
        assert!(
            store
                .save(AgentRuntimeSettingsRequest {
                    settings,
                    search_api_key: None,
                    clear_search_api_key: false
                })
                .is_err(),
            "{endpoint}"
        );
    }
    let mut settings = store.settings();
    settings.search_provider = "searxng".into();
    settings.search_endpoint = "http://127.0.0.1:8080/search".into();
    store
        .save(AgentRuntimeSettingsRequest {
            settings: settings.clone(),
            search_api_key: None,
            clear_search_api_key: false,
        })
        .unwrap();
    settings.prices = vec![LlmPrice {
        input_usd_per_million: f64::NAN,
        ..price()
    }];
    assert!(
        store
            .save(AgentRuntimeSettingsRequest {
                settings,
                search_api_key: None,
                clear_search_api_key: false
            })
            .is_err()
    );
    assert!(
        store
            .save(AgentRuntimeSettingsRequest {
                settings: store.settings(),
                search_api_key: Some("x".into()),
                clear_search_api_key: true
            })
            .is_err()
    );
}

#[test]
fn failed_atomic_save_preserves_active_settings_and_reports_storage_failure() {
    let directory = Directory::new();
    let store = RuntimeStore::open(&directory.0).unwrap();
    let before = store.settings();
    std::fs::create_dir(directory.0.join("settings.json")).unwrap();
    let mut changed = before.clone();
    changed.streaming = false;
    assert!(
        store
            .save(AgentRuntimeSettingsRequest {
                settings: changed,
                search_api_key: None,
                clear_search_api_key: false
            })
            .is_err()
    );
    assert_eq!(store.settings(), before);
    assert!(!store.snapshot().storage_available);
    assert!(std::fs::read_dir(&directory.0).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
}

fn record(index: u64, status: &str) -> serde_json::Value {
    json!({"id":format!("call-{index}"),"started_at_ms":index,"provider":"fixture","api_format":"openai_chat",
      "base_url":"https://example.test/v1","model":"chat","operation":"agent","status":status,"latency_ms":5,
      "first_token_ms":1,"usage":{"input_tokens":10,"output_tokens":2,"cache_read_tokens":0,"cache_write_tokens":0,"reasoning_tokens":1},
      "estimated_cost_microusd":null})
}

#[test]
fn recent_records_are_bounded_but_date_and_model_totals_cover_history() {
    let directory = Directory::new();
    let lines = (1..=240)
        .map(|index| record(index, "completed").to_string() + "\n")
        .collect::<String>();
    std::fs::write(directory.0.join("usage-00000000.jsonl"), lines).unwrap();
    let store = RuntimeStore::open(&directory.0).unwrap();
    let mut settings = store.settings();
    settings.prices = vec![price()];
    store
        .save(AgentRuntimeSettingsRequest {
            settings,
            search_api_key: None,
            clear_search_api_key: false,
        })
        .unwrap();
    let snapshot = store.usage(&UsageQuery::default());
    assert_eq!(snapshot.totals.calls, 240);
    assert_eq!(snapshot.totals.input_tokens, 2400);
    assert_eq!(snapshot.totals.estimated_cost_microusd, 9600);
    assert_eq!(snapshot.records.len(), 200);
    assert_eq!(snapshot.records[0].started_at_ms, 240);
    assert!(snapshot.truncated);
    let selected = store.usage(&UsageQuery {
        since_ms: Some(10),
        until_ms: Some(20),
        provider: Some("fixture".into()),
        model: Some("chat".into()),
    });
    assert_eq!(selected.totals.calls, 11);
    assert_eq!(selected.groups[0].totals.calls, 11);
    assert!(!selected.truncated);
    assert_eq!(
        store
            .usage(&UsageQuery {
                model: Some("other".into()),
                ..Default::default()
            })
            .totals
            .calls,
        0
    );
}

#[test]
fn restart_marks_pending_interrupted_without_duplicating_completed_calls() {
    let directory = Directory::new();
    std::fs::write(
        directory.0.join("usage-00000000.jsonl"),
        record(1, "completed").to_string() + "\n",
    )
    .unwrap();
    std::fs::write(
        directory.0.join("pending.json"),
        json!({"schema":1,"records":[record(1,"running"),record(2,"running")]}).to_string(),
    )
    .unwrap();
    let store = RuntimeStore::open(&directory.0).unwrap();
    let snapshot = store.usage(&UsageQuery::default());
    assert_eq!(snapshot.totals.calls, 2);
    assert_eq!(snapshot.records[0].status, "interrupted");
    drop(store);
    assert_eq!(
        RuntimeStore::open(&directory.0)
            .unwrap()
            .usage(&UsageQuery::default())
            .totals
            .calls,
        2
    );
}

#[test]
fn malformed_settings_or_ledger_are_reported_and_never_overwritten() {
    for file in ["settings.json", "pending.json", "usage-00000000.jsonl"] {
        let directory = Directory::new();
        std::fs::write(directory.0.join(file), b"{broken fixture").unwrap();
        assert!(RuntimeStore::open(&directory.0).is_err(), "{file}");
        assert_eq!(
            std::fs::read(directory.0.join(file)).unwrap(),
            b"{broken fixture"
        );
    }
}

#[test]
fn corrupted_or_missing_segments_after_open_make_incomplete_statistics_visible() {
    let directory = Directory::new();
    let segment = directory.0.join("usage-00000000.jsonl");
    std::fs::write(&segment, record(1, "completed").to_string() + "\n").unwrap();
    let store = RuntimeStore::open(&directory.0).unwrap();
    std::fs::write(segment, b"broken\n").unwrap();
    let snapshot = store.usage(&UsageQuery::default());
    assert!(!snapshot.storage_available);
    assert!(snapshot.truncated);
    assert!(!store.snapshot().storage_available);
}

#[test]
fn missing_or_oversized_history_segments_fail_without_silent_resets() {
    let directory = Directory::new();
    std::fs::write(
        directory.0.join("usage-00000001.jsonl"),
        record(1, "completed").to_string() + "\n",
    )
    .unwrap();
    assert!(RuntimeStore::open(&directory.0).is_err());
    std::fs::remove_file(directory.0.join("usage-00000001.jsonl")).unwrap();
    let oversized = std::fs::File::create(directory.0.join("usage-00000000.jsonl")).unwrap();
    oversized.set_len(4 * 1024 * 1024 + 1).unwrap();
    assert!(RuntimeStore::open(&directory.0).is_err());
}

#[test]
fn stale_activity_updates_cannot_overwrite_a_newer_agent_run() {
    let store = RuntimeStore::memory();
    let mut activity = store.activity();
    activity.run_id = Some("new-run".into());
    activity.phase = "thinking".into();
    store.set_activity(activity);
    store.update_activity_for("old-run", |activity| {
        activity.phase = "failed".into();
    });
    assert_eq!(store.activity().phase, "thinking");
    store.update_activity_for("new-run", |activity| {
        activity.phase = "receiving".into();
    });
    assert_eq!(store.activity().phase, "receiving");
}

#[test]
fn pricing_accepts_the_same_long_api_root_as_the_active_model_configuration() {
    let store = RuntimeStore::memory();
    let mut settings = store.settings();
    settings.prices = vec![LlmPrice {
        base_url: format!("https://example.test/{}", "p".repeat(3000)),
        ..price()
    }];
    store
        .save(AgentRuntimeSettingsRequest {
            settings,
            search_api_key: None,
            clear_search_api_key: false,
        })
        .unwrap();
}

#[test]
fn reconfiguration_never_pairs_a_search_key_with_another_destination() {
    use std::sync::{Arc, Barrier};
    let store = Arc::new(RuntimeStore::memory());
    let mut settings = store.settings();
    settings.search_endpoint = "https://a.example.test/search".into();
    store
        .save(AgentRuntimeSettingsRequest {
            settings,
            search_api_key: Some("a-key".into()),
            clear_search_api_key: false,
        })
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let writer_store = store.clone();
    let writer_barrier = barrier.clone();
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        for index in 0..1000 {
            let letter = if index % 2 == 0 { "a" } else { "b" };
            let mut settings = writer_store.settings();
            settings.search_endpoint = format!("https://{letter}.example.test/search");
            writer_store
                .save(AgentRuntimeSettingsRequest {
                    settings,
                    search_api_key: Some(format!("{letter}-key")),
                    clear_search_api_key: false,
                })
                .unwrap();
        }
    });
    barrier.wait();
    for _ in 0..1000 {
        let (settings, key) = store.settings_with_search_key();
        let expected = if settings.search_endpoint == "https://a.example.test/search" {
            "a-key"
        } else {
            "b-key"
        };
        assert_eq!(key.as_deref(), Some(expected));
    }
    writer.join().unwrap();
}
