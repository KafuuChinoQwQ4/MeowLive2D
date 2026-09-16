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
        ModelVersion, TrainingAuditionRequest, TrainingCreateRequest, TrainingJob, TrainingSnapshot,
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
    let mut multipart = multipart.map_err(|_| invalid("需要训练元数据与 WAV 片段"))?;
    let mut metadata = None;
    let mut audio = Vec::new();
    let mut total = 0usize;
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| invalid("训练上传内容无效或超过上限"))?
    {
        let name = field.name().unwrap_or_default().to_owned();
        if !matches!(name.as_str(), "metadata" | "audio")
            || (name == "metadata" && metadata.is_some())
            || (name == "audio" && audio.len() >= 32)
        {
            return Err(invalid("训练上传字段重复、未知或片段超过 32 个"));
        }
        let limit = if name == "metadata" {
            64 * 1024
        } else {
            2 * 1024 * 1024
        };
        let mut bytes = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| invalid("训练上传中断或超过上限"))?
        {
            total = total.saturating_add(chunk.len());
            if bytes.len() + chunk.len() > limit || total > 32 * 1024 * 1024 {
                return Err(invalid("训练素材超过大小上限"));
            }
            bytes.extend_from_slice(&chunk);
        }
        if name == "metadata" {
            metadata = Some(
                serde_json::from_slice::<TrainingCreateRequest>(&bytes)
                    .map_err(|_| invalid("训练元数据格式无效"))?,
            );
        } else {
            audio.push(bytes);
        }
    }
    let metadata = metadata.ok_or_else(|| invalid("缺少训练元数据"))?;
    if !metadata.reviewed || audio.len() < 2 || audio.len() != metadata.clips.len() {
        return Err(invalid("请提供至少两个片段，并逐片核对文本后确认训练"));
    }
    let resources = state.resources.clone();
    let voice_id = metadata.voice_id.clone();
    let valid =
        blocking(move || resources.snapshot().voices.iter().any(|v| v.id == voice_id)).await?;
    if !valid {
        return Err(invalid("请选择已上传参考音频的音色"));
    }
    let lease = state.acquire_gpu().await?;
    ensure_training_released(&state).await?;
    *state.measurement.lock().await = None;
    let parameters = TrainingParameters {
        name: metadata.name,
        voice_id: metadata.voice_id,
        sovits_epochs: metadata.sovits_epochs,
        gpt_epochs: metadata.gpt_epochs,
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
        match training.create(parameters, clips) {
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

async fn ensure_training_released(state: &AppState) -> Result<(), ApiError> {
    let mut urls = vec![state.config.speech.base_url.as_str()];
    if state.config.llm.mode == "local" {
        urls.push(state.config.llm.base_url.as_str());
    }
    for url in urls {
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
                    "请先停止本机 TTS 和本地 LLM 推理进程以释放显存，再开始训练",
                ));
            }
        }
    }
    let gpu = meowlive_adapters::runtime::sample_gpu()
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

#[cfg(test)]
mod tests {
    use super::wav;
    use meowlive_application::ports::speech::PcmAudio;

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
}
