#[allow(dead_code)]
#[path = "support/http.rs"]
mod http;

use std::{io::Cursor, sync::Arc, time::Duration};

use axum::{Json, Router, routing::post};
use meowlive_adapters::{
    speech::{
        gpt_sovits::{GptSovits, GptSovitsConfig},
        resource_synthesizer::{ResourceSynthesizer, ResourceSynthesizerConfig},
    },
    storage::resources::{FileResourceStore, FileResourceStoreConfig},
};
use meowlive_application::{
    ports::speech::{SpeechSynthesizer, SynthesisRequest},
    resources::ResourceLibrary,
};
use tokio::sync::Mutex;

fn wav() -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(
        &mut bytes,
        hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for index in 0..24000 {
        writer
            .write_sample(if index == 0 { 1_i16 } else { 0 })
            .unwrap();
    }
    writer.finalize().unwrap();
    bytes.into_inner()
}

#[tokio::test]
async fn uploaded_voices_send_their_own_engine_reference_and_default_still_delegates() {
    let paths = Arc::new(Mutex::new(Vec::new()));
    let seen = paths.clone();
    let response_wav = wav();
    let router = Router::new().route(
        "/tts",
        post(move |Json(body): Json<serde_json::Value>| {
            let seen = seen.clone();
            let audio = response_wav.clone();
            async move {
                seen.lock().await.push(body);
                audio
            }
        }),
    );
    let (url, task) = http::engine(router).await;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/resource-store-tests")
        .join(format!("meowlive-synth-{}", uuid::Uuid::new_v4()));
    let library = Arc::new(
        ResourceLibrary::open(Arc::new(
            FileResourceStore::new(FileResourceStoreConfig {
                storage_root: root.clone(),
                engine_root: "/engine/shared".into(),
            })
            .unwrap(),
        ))
        .unwrap(),
    );
    let one = library.create_voice("one", "zh", "一", &wav()).unwrap();
    let two = library.create_voice("two", "en", "two", &wav()).unwrap();
    let default = Arc::new(
        GptSovits::new(GptSovitsConfig {
            base_url: url.clone(),
            reference_audio: "/engine/default.wav".into(),
            prompt_text: "default".into(),
            prompt_language: "zh".into(),
            text_language: "zh".into(),
            timeout: Duration::from_secs(2),
            max_audio_bytes: 256 * 1024,
        })
        .unwrap(),
    );
    let synth = ResourceSynthesizer::new(
        default,
        library,
        ResourceSynthesizerConfig {
            base_url: url,
            timeout: Duration::from_secs(2),
            max_audio_bytes: 256 * 1024,
        },
    )
    .unwrap();
    for voice_id in ["default", one.id.as_str(), two.id.as_str()] {
        synth
            .synthesize(SynthesisRequest {
                text: "你好，hello".into(),
                voice_id: voice_id.into(),
            })
            .await
            .unwrap();
    }
    let seen = paths.lock().await.clone();
    assert_eq!(seen[0]["ref_audio_path"], "/engine/default.wav");
    assert_ne!(seen[1]["ref_audio_path"], seen[2]["ref_audio_path"]);
    assert!(
        seen[1]["ref_audio_path"]
            .as_str()
            .unwrap()
            .starts_with("/engine/shared/references/")
    );
    assert_eq!(seen[1]["prompt_lang"], "zh");
    assert_eq!(seen[2]["prompt_lang"], "en");
    // A reference voice's language must not reinterpret the target Chinese text as English.
    assert_eq!(seen[1]["text_lang"], "auto");
    assert_eq!(seen[2]["text_lang"], "auto");
    task.abort();
    std::fs::remove_dir_all(root).unwrap();
}
