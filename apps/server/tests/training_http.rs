mod support;
use meowlive_server::transport::http::router;
use serde_json::json;

#[tokio::test]
async fn disabled_training_is_visible_and_never_accepts_a_version() {
    let state = support::state();
    let (code, snapshot) =
        support::request(router(state.clone()), "GET", "/api/training", json!({})).await;
    assert_eq!(code, 200);
    assert_eq!(snapshot["enabled"], false);
    assert_eq!(snapshot["versions"], json!([]));
    for path in [
        "/api/training/activate",
        "/api/training/cancel",
        "/api/training/save",
        "/api/training/delete",
    ] {
        let (code, _) =
            support::request(router(state.clone()), "POST", path, json!({"id":"missing"})).await;
        assert_eq!(code, 409);
    }
}
#[tokio::test]
async fn cloud_preset_is_not_marked_as_verified_offline() {
    let state = support::state();
    let (code, snapshot) = support::request(
        router(state.clone()),
        "GET",
        "/api/runtime/preset",
        json!({}),
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(snapshot["mode"], "cloud");
    assert_eq!(snapshot["verified"], false);
    let (code, _) = support::request(
        router(state),
        "POST",
        "/api/runtime/measure",
        json!({"voice_id":"default","text":"你好"}),
    )
    .await;
    assert_eq!(code, 409);
}

struct DeleteFixture {
    root: std::path::PathBuf,
    state: meowlive_server::state::AppState,
    first: String,
    second: String,
}
impl DeleteFixture {
    fn new() -> Self {
        Self::for_voice("default")
    }
    fn for_voice(voice_id: &str) -> Self {
        use meowlive_adapters::training::FileTrainingStore;
        use meowlive_application::ports::training::{
            PreparedTrainingJob, TrainingEngine, TrainingProgress, TrainingStore,
        };
        use meowlive_application::training::TrainingManager;
        use meowlive_domain::training::*;
        use std::sync::{Arc, atomic::AtomicBool};
        struct NoEngine;
        impl TrainingEngine for NoEngine {
            fn run(
                &self,
                _: &PreparedTrainingJob,
                _: &AtomicBool,
                _: &mut dyn FnMut(TrainingProgress),
            ) -> Result<ArtifactPair, TrainingError> {
                panic!("deletion must not launch training");
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .canonicalize()
            .unwrap()
            .join(format!("training-delete-http-{}", uuid::Uuid::new_v4()));
        let store = Arc::new(FileTrainingStore::open(&root).unwrap());
        let first = store.new_job_id();
        let second = store.new_job_id();
        let jobs = [&first, &second]
            .into_iter()
            .map(|id| {
                std::fs::create_dir(root.join("jobs").join(id)).unwrap();
                TrainingJob {
                    id: id.clone(),
                    parameters: TrainingParameters {
                        name: "已保存训练版本".into(),
                        voice_id: voice_id.into(),
                        sovits_epochs: 1,
                        gpt_epochs: 1,
                        performance: TrainingPerformance::default(),
                    },
                    state: TrainingState::Succeeded,
                    clip_count: 2,
                    total_bytes: 0,
                    created_at_ms: 1,
                    updated_at_ms: 1,
                    progress: 100,
                    message: "训练完成".into(),
                    artifacts: Some(ArtifactPair::v2()),
                    auditioned: true,
                    saved: true,
                }
            })
            .collect();
        store
            .save(&TrainingCatalog {
                jobs,
                active_versions: [(voice_id.into(), first.clone())].into(),
            })
            .unwrap();
        let mut state = support::state();
        state.training = Arc::new(TrainingManager::open(store, Arc::new(NoEngine)).unwrap());
        Self {
            root,
            state,
            first,
            second,
        }
    }
}
impl Drop for DeleteFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn deleting_saved_version_clears_active_reference_and_returns_remaining_snapshot() {
    let fixture = DeleteFixture::new();
    fixture.state.resources.select_voice("default").unwrap();
    let (code, snapshot) = support::request(
        router(fixture.state.clone()),
        "POST",
        "/api/training/delete",
        json!({"id": fixture.first}),
    )
    .await;
    assert_eq!(code, 200, "{snapshot}");
    assert_eq!(snapshot["jobs"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["versions"][0]["id"], fixture.second);
    assert_eq!(snapshot["versions"][0]["saved"], true);
    assert!(fixture.state.training.snapshot().active_versions.is_empty());
    assert!(
        fixture
            .state
            .resources
            .snapshot()
            .active_voice_id
            .is_empty()
    );
    assert!(!fixture.root.join("jobs").join(&fixture.first).exists());
    assert!(fixture.root.join("jobs").join(&fixture.second).exists());
    let (code, _) = support::request(
        router(fixture.state.clone()),
        "POST",
        "/api/training/delete",
        json!({"id": fixture.first}),
    )
    .await;
    assert_eq!(code, 200);
}

#[tokio::test]
async fn deletion_rejects_path_payloads_and_shutdown_without_changing_versions() {
    let fixture = DeleteFixture::new();
    for payload in [
        json!({"id":"../outside"}),
        json!({"path":fixture.root}),
        json!({"id":fixture.first,"path":"outside"}),
    ] {
        let (code, _) = support::request(
            router(fixture.state.clone()),
            "POST",
            "/api/training/delete",
            payload,
        )
        .await;
        assert_eq!(code, 400);
    }
    fixture.state.resources.select_voice("default").unwrap();
    fixture.state.training.shutdown();
    let (code, _) = support::request(
        router(fixture.state.clone()),
        "POST",
        "/api/training/delete",
        json!({"id":fixture.first}),
    )
    .await;
    assert_eq!(code, 409);
    assert_eq!(fixture.state.training.snapshot().jobs.len(), 2);
    assert_eq!(
        fixture.state.resources.snapshot().active_voice_id,
        "default"
    );
}

#[tokio::test]
async fn deletion_preserves_selection_for_inactive_versions_or_other_reference_voices() {
    for (trained_voice, active_version) in [("default", false), ("other-voice", true)] {
        let fixture = DeleteFixture::for_voice(trained_voice);
        fixture.state.resources.select_voice("default").unwrap();
        let id = if active_version {
            &fixture.first
        } else {
            &fixture.second
        };
        let (code, body) = support::request(
            router(fixture.state.clone()),
            "POST",
            "/api/training/delete",
            json!({"id":id}),
        )
        .await;
        assert_eq!(code, 200, "{body}");
        assert_eq!(
            fixture.state.resources.snapshot().active_voice_id,
            "default"
        );
    }
}

#[tokio::test]
async fn failed_training_catalog_commit_preserves_model_and_leaves_selection_safely_empty() {
    let fixture = DeleteFixture::new();
    fixture.state.resources.select_voice("default").unwrap();
    std::fs::rename(
        fixture.root.join("catalog.json"),
        fixture.root.join("catalog-backup.json"),
    )
    .unwrap();
    std::fs::create_dir(fixture.root.join("catalog.json")).unwrap();
    let (code, body) = support::request(
        router(fixture.state.clone()),
        "POST",
        "/api/training/delete",
        json!({"id":fixture.first}),
    )
    .await;
    assert_eq!(code, 500);
    assert!(
        fixture
            .state
            .resources
            .snapshot()
            .active_voice_id
            .is_empty()
    );
    assert!(body["message"].as_str().unwrap().contains("重新选择"));
    assert_eq!(fixture.state.training.snapshot().jobs.len(), 2);
    assert_eq!(
        fixture.state.training.snapshot().active_versions["default"],
        fixture.first
    );
    assert!(fixture.root.join("jobs").join(&fixture.first).is_dir());
}

#[tokio::test]
async fn committed_deletion_clears_selection_even_when_directory_cleanup_fails() {
    let fixture = DeleteFixture::new();
    fixture.state.resources.select_voice("default").unwrap();
    let job = fixture.root.join("jobs").join(&fixture.first);
    std::fs::remove_dir(&job).unwrap();
    std::fs::write(&job, "cleanup blocked").unwrap();
    let (code, body) = support::request(
        router(fixture.state.clone()),
        "POST",
        "/api/training/delete",
        json!({"id":fixture.first}),
    )
    .await;
    assert_eq!(code, 500);
    assert!(body["message"].as_str().unwrap().contains("文件清理失败"));
    assert_eq!(fixture.state.training.snapshot().jobs.len(), 1);
    assert!(
        fixture
            .state
            .resources
            .snapshot()
            .active_voice_id
            .is_empty()
    );
}

#[tokio::test]
async fn failed_resource_selection_commit_keeps_active_model_and_allows_retry() {
    use meowlive_adapters::storage::resources::{FileResourceStore, FileResourceStoreConfig};
    use meowlive_application::resources::ResourceLibrary;
    use std::sync::Arc;
    let mut fixture = DeleteFixture::new();
    let root = fixture.root.join("resources");
    fixture.state.resources = Arc::new(
        ResourceLibrary::open(Arc::new(
            FileResourceStore::new(FileResourceStoreConfig {
                storage_root: root.clone(),
                engine_root: root.to_str().unwrap().into(),
            })
            .unwrap(),
        ))
        .unwrap(),
    );
    fixture.state.resources.select_voice("default").unwrap();
    std::fs::rename(root.join("catalog.json"), root.join("catalog-backup.json")).unwrap();
    std::fs::create_dir(root.join("catalog.json")).unwrap();
    let (code, body) = support::request(
        router(fixture.state.clone()),
        "POST",
        "/api/training/delete",
        json!({"id":fixture.first}),
    )
    .await;
    assert_eq!(code, 500);
    assert_eq!(body["code"], "training_selection_clear_failed");
    assert!(body["message"].as_str().unwrap().contains("版本未删除"));
    assert_eq!(fixture.state.training.snapshot().jobs.len(), 2);
    assert_eq!(
        fixture.state.resources.snapshot().active_voice_id,
        "default"
    );
    assert_eq!(
        fixture.state.training.snapshot().active_versions["default"],
        fixture.first
    );
    assert!(fixture.root.join("jobs").join(&fixture.first).is_dir());
    std::fs::remove_dir(root.join("catalog.json")).unwrap();
    std::fs::rename(root.join("catalog-backup.json"), root.join("catalog.json")).unwrap();
    let (code, _) = support::request(
        router(fixture.state.clone()),
        "POST",
        "/api/training/delete",
        json!({"id":fixture.first}),
    )
    .await;
    assert_eq!(code, 200);
    assert!(
        fixture
            .state
            .resources
            .snapshot()
            .active_voice_id
            .is_empty()
    );
    assert_eq!(fixture.state.training.snapshot().jobs.len(), 1);
}

#[tokio::test]
async fn transcription_is_explicitly_unavailable_without_training_configuration() {
    let (code, body) = support::request(
        router(support::state()),
        "POST",
        "/api/training/transcribe",
        json!({}),
    )
    .await;
    assert_eq!(code, 409, "{body}");
}

async fn transcription_upload(
    state: meowlive_server::state::AppState,
    metadata: serde_json::Value,
    fields: &[(&str, &[u8])],
) -> (u16, serde_json::Value) {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let mut body = format!("--transcription-test\r\nContent-Disposition: form-data; name=\"metadata\"\r\n\r\n{metadata}\r\n").into_bytes();
    for (name, audio) in fields {
        body.extend_from_slice(format!("--transcription-test\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"clip.wav\"\r\nContent-Type: audio/wav\r\n\r\n").as_bytes());
        body.extend_from_slice(audio);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--transcription-test--\r\n");
    let response = router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/training/transcribe")
                .header(
                    "content-type",
                    "multipart/form-data; boundary=transcription-test",
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn transcription_http_returns_editable_text_and_rejects_invalid_uploads() {
    use meowlive_application::ports::training::{
        PreparedTrainingJob, TrainingEngine, TrainingProgress,
    };
    use meowlive_domain::training::{ArtifactPair, TrainingClip, TrainingError};
    use std::sync::{Arc, atomic::AtomicBool};
    struct Recognizer;
    impl TrainingEngine for Recognizer {
        fn run(
            &self,
            _: &PreparedTrainingJob,
            _: &AtomicBool,
            _: &mut dyn FnMut(TrainingProgress),
        ) -> Result<ArtifactPair, TrainingError> {
            panic!("text previews must not launch training");
        }
        fn transcribe(&self, clip: &TrainingClip, _: &AtomicBool) -> Result<String, TrainingError> {
            assert!(clip.text.is_empty());
            assert_eq!(clip.language, "zh");
            if clip.wav == b"engine-error" {
                return Err(TrainingError::Engine(
                    "自动提取文本失败，请手动填写文本".into(),
                ));
            }
            Ok("提取后可校对的文本。".into())
        }
    }
    let mut fixture = DeleteFixture::new();
    fixture.state.training = Arc::new(
        meowlive_application::training::TrainingManager::open(
            Arc::new(meowlive_adapters::training::FileTrainingStore::open(&fixture.root).unwrap()),
            Arc::new(Recognizer),
        )
        .unwrap(),
    );
    let (code, body) = transcription_upload(
        fixture.state.clone(),
        json!({"language":"zh"}),
        &[("audio", b"wave")],
    )
    .await;
    assert_eq!(code, 200, "{body}");
    assert_eq!(
        body,
        json!({"text":"提取后可校对的文本。", "language":"zh"})
    );
    assert!(!fixture.state.training.snapshot().busy);
    assert_eq!(fixture.state.training.snapshot().jobs.len(), 2);
    for (metadata, fields) in [
        (
            json!({"language":"xx"}),
            vec![("audio", b"wave".as_slice())],
        ),
        (
            json!({"language":"zh", "path":"/private"}),
            vec![("audio", b"wave".as_slice())],
        ),
        (
            json!({"language":"zh"}),
            vec![("audio", b"wave".as_slice()), ("audio", b"wave".as_slice())],
        ),
        (
            json!({"language":"zh"}),
            vec![("unknown", b"wave".as_slice())],
        ),
        (json!({"language":"zh"}), vec![]),
        (json!({"language":"zh"}), vec![("audio", b"".as_slice())]),
    ] {
        let (code, body) = transcription_upload(fixture.state.clone(), metadata, &fields).await;
        assert_eq!(code, 400, "{body}");
    }
    let oversized = vec![1; 2 * 1024 * 1024 + 1];
    let (code, _) = transcription_upload(
        fixture.state.clone(),
        json!({"language":"zh"}),
        &[("audio", &oversized)],
    )
    .await;
    assert_eq!(code, 400);
    let (code, body) = transcription_upload(
        fixture.state.clone(),
        json!({"language":"zh"}),
        &[("audio", b"engine-error")],
    )
    .await;
    assert_eq!(code, 500);
    assert!(body["message"].as_str().unwrap().contains("手动填写"));
    assert!(!fixture.state.training.snapshot().busy);
}
