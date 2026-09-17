//! 微调任务的有界素材导入、进程启动、版本试听、保存与启用。
use crate::{state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{Multipart, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use meowlive_application::training::TrainingManager;
use meowlive_domain::training::{TrainingClip, TrainingError, TrainingParameters, TrainingState};
use meowlive_protocol::{
    resources::ResourceSelection,
    training::{
        ModelVersion, TrainingAuditionRequest, TrainingCreateRequest, TrainingJob,
        TrainingSnapshot, TrainingTextMode, TrainingTranscribeRequest, TrainingTranscription,
    },
};

pub async fn status(State(state): State<AppState>) -> Result<Json<TrainingSnapshot>, ApiError> {
    let training = state.training.clone();
    let mut value = blocking(move || snapshot(&training)).await?;
    value.busy |= state.gpu_busy.load(std::sync::atomic::Ordering::Acquire)
        || state
            .model_synthesizer
            .as_ref()
            .is_some_and(|s| s.is_busy());
    Ok(Json(value))
}
fn snapshot(training: &TrainingManager) -> TrainingSnapshot {
    let state = training.snapshot();
    let versions = state
        .jobs
        .iter()
        .filter(|job| job.state == TrainingState::Succeeded)
        .map(|job| ModelVersion {
            id: job.id.clone(),
            job_id: job.id.clone(),
            voice_id: job.parameters.voice_id.clone(),
            name: job.parameters.name.clone(),
            engine: "gpt-sovits".into(),
            model_version: "v2".into(),
            auditioned: job.auditioned,
            saved: job.saved,
            active: state.active_versions.get(&job.parameters.voice_id) == Some(&job.id),
            available: training.resolve_version(&job.id).is_ok(),
            created_at_ms: job.created_at_ms,
        })
        .collect();
    TrainingSnapshot {
        enabled: state.configured,
        busy: state.busy,
        jobs: state.jobs.iter().map(job_dto).collect(),
        versions,
    }
}
fn job_dto(job: &meowlive_domain::training::TrainingJob) -> TrainingJob {
    TrainingJob {
        id: job.id.clone(),
        name: job.parameters.name.clone(),
        voice_id: job.parameters.voice_id.clone(),
        status: if job.state == TrainingState::Succeeded {
            "completed".into()
        } else {
            job.state.as_str().into()
        },
        progress: job.progress,
        message: job.message.clone(),
        clip_count: job.clip_count as u32,
        created_at_ms: job.created_at_ms,
        updated_at_ms: job.updated_at_ms,
        version_id: job.artifacts.as_ref().map(|_| job.id.clone()),
        performance: meowlive_protocol::training::TrainingPerformance {
            batch_size: job.parameters.performance.batch_size,
            data_workers: job.parameters.performance.data_workers,
            cpu_threads: job.parameters.performance.cpu_threads,
            gpu_index: job.parameters.performance.gpu_index,
            low_memory: job.parameters.performance.low_memory,
        },
    }
}

pub async fn create(
    State(state): State<AppState>,
    multipart: Result<Multipart, axum::extract::multipart::MultipartRejection>,
) -> Result<(StatusCode, Json<TrainingJob>), ApiError> {
    let _request = state
        .resource_requests
        .clone()
        .try_acquire_owned()
        .map_err(|_| conflict("upload_busy", "已有资源上传正在进行"))?;
    if !state.training.snapshot().configured {
        return Err(training_error(TrainingError::Disabled));
    }
    let (metadata, audio) = read_upload(multipart, 32, 64 * 1024).await?;
    let metadata: TrainingCreateRequest =
        serde_json::from_slice(&metadata).map_err(|_| invalid("训练元数据格式无效"))?;
    validate_create_metadata(&metadata, audio.len())?;
    let performance = meowlive_domain::training::TrainingPerformance {
        batch_size: metadata.performance.batch_size,
        data_workers: metadata.performance.data_workers,
        cpu_threads: metadata.performance.cpu_threads,
        gpu_index: metadata.performance.gpu_index,
        low_memory: metadata.performance.low_memory,
    };
    performance.validate().map_err(training_error)?;
    // Keep reference validation and catalog acceptance atomic with resource
    // deletion. Release this fence once the durable job owns the reference.
    let edit = state
        .resource_edits
        .clone()
        .try_lock_owned()
        .map_err(|_| conflict("resource_busy", "角色资源正在更新，请稍后重试训练"))?;
    let resources = state.resources.clone();
    let voice_id = metadata.voice_id.clone();
    let valid =
        blocking(move || resources.snapshot().voices.iter().any(|v| v.id == voice_id)).await?;
    if !valid {
        return Err(invalid("请选择已上传参考音频的音色"));
    }
    let lease = state.acquire_gpu().await?;
    ensure_training_released(&state, performance.gpu_index).await?;
    *state.measurement.lock().await = None;
    let parameters = TrainingParameters {
        name: metadata.name,
        voice_id: metadata.voice_id,
        sovits_epochs: metadata.sovits_epochs,
        gpt_epochs: metadata.gpt_epochs,
        performance,
    };
    let clips = metadata
        .clips
        .into_iter()
        .zip(audio)
        .map(|(meta, wav)| TrainingClip {
            text: meta.text,
            language: meta.language,
            wav,
        })
        .collect();
    let training = state.training.clone();
    let (send, receive) = tokio::sync::oneshot::channel();
    // The task, not the HTTP connection, owns the lease and accepted training lifecycle.
    tokio::task::spawn_blocking(move || {
        let _lease = lease;
        let accepted = training.create(parameters, clips);
        drop(edit);
        match accepted {
            Ok(job) => {
                let _ = send.send(Ok(job_dto(&job)));
                let _ = training.run(&job.id);
            }
            Err(error) => {
                let _ = send.send(Err(training_error(error)));
            }
        }
    });
    let job = receive.await.map_err(|_| internal())??;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

fn validate_create_metadata(
    metadata: &TrainingCreateRequest,
    audio_count: usize,
) -> Result<(), ApiError> {
    if audio_count < 2 || audio_count != metadata.clips.len() {
        return Err(invalid("请提供至少两个音频片段，文本和音频数量须一致"));
    }
    match metadata.text_mode {
        TrainingTextMode::AudioOnly if metadata.clips.iter().any(|clip| !clip.text.is_empty()) => {
            Err(invalid("仅语音训练的文本须留空，系统会自动提取文本"))
        }
        TrainingTextMode::ReviewedText
            if !metadata.reviewed
                || metadata
                    .clips
                    .iter()
                    .any(|clip| clip.text.trim().is_empty()) =>
        {
            Err(invalid("请补全片段文本，逐片核对后确认训练"))
        }
        _ => Ok(()),
    }
}

pub async fn transcribe(
    State(state): State<AppState>,
    multipart: Result<Multipart, axum::extract::multipart::MultipartRejection>,
) -> Result<Json<TrainingTranscription>, ApiError> {
    let request = state
        .resource_requests
        .clone()
        .try_acquire_owned()
        .map_err(|_| conflict("upload_busy", "已有资源上传正在进行"))?;
    if !state.training.snapshot().configured {
        return Err(training_error(TrainingError::Disabled));
    }
    let (metadata, mut audio) = read_upload(multipart, 1, 1024).await?;
    let metadata: TrainingTranscribeRequest =
        serde_json::from_slice(&metadata).map_err(|_| invalid("自动提取文本需要有效的音频语言"))?;
    if audio.len() != 1 {
        return Err(invalid("每次自动提取文本需要一个音频片段"));
    }
    let clip = TrainingClip {
        text: String::new(),
        language: metadata.language.clone(),
        wav: audio.remove(0),
    };
    clip.validate().map_err(training_error)?;
    // CPU recognition shares model admission so shutdown and other operations stay serialized.
    // The blocking task owns both permits even if the browser disconnects.
    let lease = state.acquire_model_selection().await?;
    let training = state.training.clone();
    let text = blocking(move || {
        let _request = request;
        let _lease = lease;
        training.transcribe(clip)
    })
    .await?
    .map_err(training_error)?;
    Ok(Json(TrainingTranscription {
        text,
        language: metadata.language,
    }))
}

async fn read_upload(
    multipart: Result<Multipart, axum::extract::multipart::MultipartRejection>,
    max_audio: usize,
    max_metadata: usize,
) -> Result<(Vec<u8>, Vec<Vec<u8>>), ApiError> {
    let mut multipart = multipart.map_err(|_| invalid("需要元数据与 WAV 片段"))?;
    let mut metadata = None;
    let mut audio = Vec::new();
    let mut total = 0usize;
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| invalid("音频上传内容无效或超过上限"))?
    {
        let name = field.name().unwrap_or_default().to_owned();
        if !matches!(name.as_str(), "metadata" | "audio")
            || (name == "metadata" && metadata.is_some())
            || (name == "audio" && audio.len() >= max_audio)
        {
            return Err(invalid("音频上传字段重复、未知或片段数量超过上限"));
        }
        let limit = if name == "metadata" {
            max_metadata
        } else {
            2 * 1024 * 1024
        };
        let mut bytes = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| invalid("音频上传中断或超过上限"))?
        {
            total = total.saturating_add(chunk.len());
            if bytes.len() + chunk.len() > limit || total > 32 * 1024 * 1024 {
                return Err(invalid("音频素材超过大小上限"));
            }
            bytes.extend_from_slice(&chunk);
        }
        if name == "metadata" {
            metadata = Some(bytes);
        } else {
            audio.push(bytes);
        }
    }
    Ok((metadata.ok_or_else(|| invalid("缺少音频元数据"))?, audio))
}

pub async fn cancel(
    State(state): State<AppState>,
    body: Result<Json<ResourceSelection>, JsonRejection>,
) -> Result<Json<TrainingSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("需要任务 ID"))?;
    let training = state.training.clone();
    blocking(move || training.cancel(&request.id))
        .await?
        .map_err(training_error)?;
    status(State(state)).await
}
pub async fn activate(
    State(state): State<AppState>,
    body: Result<Json<ResourceSelection>, JsonRejection>,
) -> Result<Json<TrainingSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("需要版本 ID"))?;
    if !state.training.snapshot().configured {
        return Err(training_error(TrainingError::Disabled));
    }
    if state.model_synthesizer.is_none() {
        return Err(conflict(
            "inference_unmanaged",
            "请启动项目受管推理实例后启用训练权重",
        ));
    }
    let lease = state.acquire_model_selection().await?;
    let training = state.training.clone();
    blocking(move || {
        let _lease = lease;
        training.activate(&request.id)
    })
    .await?
    .map_err(training_error)?;
    *state.measurement.lock().await = None;
    status(State(state)).await
}
pub async fn save(
    State(state): State<AppState>,
    body: Result<Json<ResourceSelection>, JsonRejection>,
) -> Result<Json<TrainingSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("需要版本 ID"))?;
    let training = state.training.clone();
    blocking(move || training.save_version(&request.id))
        .await?
        .map_err(training_error)?;
    status(State(state)).await
}
pub async fn delete(
    State(state): State<AppState>,
    body: Result<Json<ResourceSelection>, JsonRejection>,
) -> Result<Json<TrainingSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| invalid("需要训练版本 ID"))?;
    if !state.training.snapshot().configured {
        return Err(training_error(TrainingError::Disabled));
    }
    let edit = state
        .resource_edits
        .clone()
        .try_lock_owned()
        .map_err(|_| conflict("resource_busy", "角色资源正在更新，请稍后重试删除"))?;
    let lease = state.acquire_model_selection().await?;
    let training = state.training.clone();
    let resources = state.resources.clone();
    let (changed, result) = blocking(move || {
        let _edit = edit;
        let _lease = lease;
        if let Err(error) = training.check_delete(&request.id) {
            return (false, Err(training_error(error)));
        }
        let before = training.snapshot();
        let selected = resources.snapshot().active_voice_id;
        let clear_selection = before
            .active_versions
            .get(&selected)
            .is_some_and(|id| id == &request.id);
        if clear_selection
            && resources
                .clear_voice_selection_if_current(&selected)
                .is_err()
        {
            return (
                false,
                Err(ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "training_selection_clear_failed",
                    "取消当前音色选择失败，训练版本未删除，请检查资源存储后重试删除",
                )),
            );
        }
        let result = training.delete(&request.id);
        let committed = before.jobs.iter().any(|job| job.id == request.id)
            && !training
                .snapshot()
                .jobs
                .iter()
                .any(|job| job.id == request.id);
        let result = result.map_err(|error| {
            let mut response = training_error(error);
            if clear_selection && !committed {
                response
                    .2
                    .push_str("；当前音色选择已取消，训练版本仍保留，可重试删除或重新选择音色");
            }
            response
        });
        (committed || clear_selection, result)
    })
    .await?;
    if changed {
        *state.measurement.lock().await = None;
    }
    result?;
    status(State(state)).await
}
pub async fn audition(
    State(state): State<AppState>,
    body: Result<Json<TrainingAuditionRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(request) = body.map_err(|_| invalid("需要版本 ID 和试听文本"))?;
    meowlive_domain::speech::SpeechText::new(&request.text).map_err(|_| invalid("试听文本无效"))?;
    let synthesizer = state.model_synthesizer.clone().ok_or_else(|| {
        conflict(
            "inference_unmanaged",
            "请启动项目受管推理实例后试听训练权重",
        )
    })?;
    let lease = state.acquire_gpu().await?;
    let training = state.training.clone();
    let bytes = tokio::spawn(async move {
        let _lease = lease;
        let audio = synthesizer
            .audition(&request.version_id, request.text)
            .await
            .map_err(|e| conflict("audition_failed", e.message))?;
        let bytes = wav(audio)?;
        blocking(move || training.mark_auditioned(&request.version_id))
            .await?
            .map_err(training_error)?;
        Ok::<_, ApiError>(bytes)
    })
    .await
    .map_err(|_| internal())??;
    Ok((
        [
            (header::CONTENT_TYPE, "audio/wav"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}
pub(crate) fn wav(
    audio: meowlive_application::ports::speech::PcmAudio,
) -> Result<Vec<u8>, ApiError> {
    if !(8000..=48000).contains(&audio.sample_rate)
        || !matches!(audio.channels, 1 | 2)
        || audio.samples.is_empty()
        || !audio.samples.iter().any(|sample| *sample != 0)
        || audio.samples.len() > 4 * 1024 * 1024
        || audio.samples.len() % usize::from(audio.channels) != 0
    {
        return Err(conflict("invalid_audio", "试听音频格式无效"));
    }
    let size = (audio.samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(size as usize + 44);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&audio.channels.to_le_bytes());
    bytes.extend_from_slice(&audio.sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(audio.sample_rate * u32::from(audio.channels) * 2).to_le_bytes());
    bytes.extend_from_slice(&(audio.channels * 2).to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&size.to_le_bytes());
    for sample in audio.samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(bytes)
}
pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| internal())
}

fn training_error(error: TrainingError) -> ApiError {
    let status = match error {
        TrainingError::Disabled | TrainingError::Busy | TrainingError::NotAuditioned => {
            StatusCode::CONFLICT
        }
        TrainingError::NotFound => StatusCode::NOT_FOUND,
        TrainingError::Capacity => StatusCode::TOO_MANY_REQUESTS,
        TrainingError::Invalid(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    ApiError::new(status, "training_failed", error.to_string())
}
fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "invalid_training", message)
}
pub(crate) fn conflict(code: &'static str, message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}
pub(crate) fn internal() -> ApiError {
    ApiError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "training_failed",
        "训练操作未完成",
    )
}

async fn ensure_training_released(state: &AppState, gpu_index: u16) -> Result<(), ApiError> {
    let managed_tts_released = match &state.model_synthesizer {
        Some(synthesizer) => synthesizer
            .model_status()
            .await
            .is_ok_and(|status| managed_tts_is_released(&status)),
        None => meowlive_adapters::speech::model_synthesizer::request_model_runtime(
            &state.config.speech.base_url,
            None,
        )
        .await
        .is_ok_and(|status| managed_tts_is_released(&status)),
    };
    let mut urls = vec![(state.config.speech.base_url.as_str(), !managed_tts_released)];
    if state.config.llm.mode == "local" {
        urls.push((state.config.llm.base_url.as_str(), true));
    }
    for (url, must_be_stopped) in urls {
        let uri = url
            .parse::<axum::http::Uri>()
            .map_err(|_| conflict("training_preflight", "本机推理地址无效"))?;
        let ip = uri
            .host()
            .unwrap_or_default()
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .map_err(|_| conflict("training_preflight", "训练仅支持回环 IP 的本机引擎"))?;
        if !ip.is_loopback() {
            return Err(conflict(
                "training_preflight",
                "训练前须使用本机推理配置并停止推理实例",
            ));
        }
        if !must_be_stopped {
            continue;
        }
        let port = uri
            .port_u16()
            .unwrap_or(if uri.scheme_str() == Some("https") {
                443
            } else {
                80
            });
        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio::net::TcpStream::connect(std::net::SocketAddr::new(ip, port)),
        )
        .await
        {
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionRefused => {}
            _ => {
                return Err(conflict(
                    "inference_running",
                    "请先关闭受管语音模型（或停止 TTS），并停止本地 LLM 以释放显存，再开始训练",
                ));
            }
        }
    }
    let gpu = meowlive_adapters::runtime::sample_gpu_index(gpu_index)
        .await
        .map_err(|e| conflict("gpu_unavailable", e))?;
    if gpu.used_mib > 512 {
        return Err(conflict(
            "gpu_occupied",
            "显存仍被其他进程占用，请释放 GPU 后训练",
        ));
    }
    Ok(())
}

fn managed_tts_is_released(
    status: &meowlive_adapters::speech::model_synthesizer::ModelRuntimeStatus,
) -> bool {
    status.supported && status.state == "unloaded"
}

#[cfg(test)]
mod tests {
    use super::wav;
    use meowlive_application::ports::speech::PcmAudio;

    #[test]
    fn only_supported_unloaded_managed_tts_can_remain_online_for_training() {
        use meowlive_adapters::speech::model_synthesizer::ModelRuntimeStatus;
        let status = |supported, state: &str| ModelRuntimeStatus {
            supported,
            state: state.into(),
            message: String::new(),
        };
        assert!(super::managed_tts_is_released(&status(true, "unloaded")));
        for value in [
            status(false, "unsupported"),
            status(true, "loaded"),
            status(true, "loading"),
            status(true, "unloading"),
            status(true, "failed"),
        ] {
            assert!(!super::managed_tts_is_released(&value));
        }
    }

    #[test]
    fn audio_only_bypasses_manual_review_but_manual_and_legacy_requests_require_text() {
        let mut value = serde_json::json!({"name":"训练", "voice_id":"voice", "gpt_epochs":1, "sovits_epochs":1,
            "reviewed":false, "text_mode":"audio_only", "clips":[{"language":"zh"},{"language":"zh"}]});
        let validate = |value: serde_json::Value| {
            super::validate_create_metadata(&serde_json::from_value(value).unwrap(), 2)
        };
        assert!(validate(value.clone()).is_ok());
        value["clips"][0]["text"] = "不应使用的文本".into();
        assert!(validate(value.clone()).is_err());
        value["clips"][0]["text"] = "".into();
        value["text_mode"] = "reviewed_text".into();
        value["reviewed"] = true.into();
        assert!(validate(value.clone()).is_err());
        value["clips"][0]["text"] = "第一段".into();
        value["clips"][1]["text"] = "第二段".into();
        assert!(validate(value.clone()).is_ok());
        value.as_object_mut().unwrap().remove("text_mode");
        value["reviewed"] = false.into();
        assert!(validate(value).is_err());
    }
    #[test]
    fn silent_audition_is_rejected_and_nonzero_pcm_is_encoded() {
        let audio = |sample| PcmAudio {
            sample_rate: 8000,
            channels: 1,
            samples: vec![sample; 8000],
        };
        assert!(wav(audio(0)).is_err());
        let encoded = wav(audio(1)).unwrap();
        let decoded = meowlive_adapters::speech::wav::decode_wav(&encoded).unwrap();
        assert_eq!(decoded.samples, vec![1; 8000]);
        assert_eq!(decoded.sample_rate, 8000);
    }

    #[tokio::test]
    async fn deleting_training_versions_is_fenced_during_audition_and_speech() {
        use meowlive_application::ports::{
            speech::{SpeechSynthesizer, SynthesisFuture, SynthesisRequest},
            training::{PreparedTrainingJob, TrainingEngine, TrainingProgress},
        };
        use meowlive_domain::training::{ArtifactPair, TrainingError};
        use std::sync::{Arc, atomic::AtomicBool};
        struct NoSpeech;
        impl SpeechSynthesizer for NoSpeech {
            fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
                panic!("deletion must not start speech");
            }
        }
        struct NoTraining;
        impl TrainingEngine for NoTraining {
            fn run(
                &self,
                _: &PreparedTrainingJob,
                _: &AtomicBool,
                _: &mut dyn FnMut(TrainingProgress),
            ) -> Result<ArtifactPair, TrainingError> {
                panic!("deletion must not start training");
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .canonicalize()
            .unwrap()
            .join(format!("delete-lease-{}", uuid::Uuid::new_v4()));
        let mut state =
            crate::state::AppState::new(crate::config::AppConfig::default(), Arc::new(NoSpeech));
        state.training = Arc::new(
            meowlive_application::training::TrainingManager::open(
                Arc::new(meowlive_adapters::training::FileTrainingStore::open(&root).unwrap()),
                Arc::new(NoTraining),
            )
            .unwrap(),
        );
        let request = || {
            Ok(axum::Json(
                meowlive_protocol::resources::ResourceSelection {
                    id: "00000000-0000-4000-8000-000000000001".into(),
                },
            ))
        };
        // Admission must take the resource fence before checking the reference
        // voice; otherwise a concurrent delete can invalidate that check.
        async fn submit_training(state: crate::state::AppState) -> axum::response::Response {
            use axum::{body::Body, http::Request};
            use tower::ServiceExt;
            let metadata = serde_json::json!({
                "name":"test", "voice_id":"missing", "reviewed":true,
                "sovits_epochs":1, "gpt_epochs":1,
                "clips":[{"text":"one", "language":"en"}, {"text":"two", "language":"en"}]
            });
            let body = format!(
                "--training-fence\r\nContent-Disposition: form-data; name=\"metadata\"\r\n\r\n{metadata}\r\n--training-fence\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"one.wav\"\r\n\r\nwav\r\n--training-fence\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"two.wav\"\r\n\r\nwav\r\n--training-fence--\r\n"
            );
            crate::transport::http::router(state)
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/training/jobs")
                        .header(
                            "content-type",
                            "multipart/form-data; boundary=training-fence",
                        )
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap()
        }
        let edit = state.resource_edits.clone().try_lock_owned().unwrap();
        assert_eq!(
            submit_training(state.clone()).await.status(),
            axum::http::StatusCode::CONFLICT
        );
        let error = super::delete(axum::extract::State(state.clone()), request())
            .await
            .unwrap_err();
        assert_eq!(error.1, "resource_busy");
        drop(edit);
        assert_eq!(
            submit_training(state.clone()).await.status(),
            axum::http::StatusCode::BAD_REQUEST
        );
        assert!(state.training.snapshot().jobs.is_empty());
        let lease = state.acquire_gpu().await.unwrap();
        let error = super::delete(axum::extract::State(state.clone()), request())
            .await
            .unwrap_err();
        use axum::response::IntoResponse;
        assert_eq!(
            error.into_response().status(),
            axum::http::StatusCode::CONFLICT
        );
        drop(lease);
        {
            let mut inner = state.inner.lock().await;
            inner.queue.set_connected(true);
            inner.queue.enqueue("pending", "hello", "default").unwrap();
        }
        let error = super::delete(axum::extract::State(state), request())
            .await
            .unwrap_err();
        assert_eq!(
            error.into_response().status(),
            axum::http::StatusCode::CONFLICT
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
