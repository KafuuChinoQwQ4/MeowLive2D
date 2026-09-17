//! 本地 LLM + TTS 顺序链路实测，采样显存并保存本次进程的验证结果。
use super::training::{blocking, conflict, internal, wav};
use crate::{state::AppState, transport::error::ApiError};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
};
use meowlive_application::ports::{llm::DecisionRequest, speech::SynthesisRequest};
use meowlive_domain::event::{EventKind, LiveEvent};
use meowlive_protocol::training::{
    RuntimeMeasureRequest, RuntimeMeasurement, RuntimePresetSnapshot,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub async fn status(State(state): State<AppState>) -> Json<RuntimePresetSnapshot> {
    let measurement = state.measurement.lock().await.clone();
    let local = state.config.llm.mode == "local";
    let verified = local && measurement.as_ref().is_some_and(|m| m.passed);
    Json(RuntimePresetSnapshot {
        mode: state.config.llm.mode.clone(),
        model: state.config.llm.model.clone(),
        local_only: local,
        verified,
        max_tokens: state.config.llm.max_tokens,
        timeout_seconds: state.config.llm.timeout_seconds as u32,
        measurement:measurement.clone(),
        message: if verified {
            "本次配置已完成 LLM→TTS 实测；Windows VTS/OBS 联合资源仍需单独测量"
        } else if local && measurement.is_some() {
            "实测未通过：要求显存峰值不超过 5120 MiB、余量至少 512 MiB、总耗时不超过 30 秒，且采样完整"
        } else if local {
            "本地预设尚未验证，请先启动本地 LLM 和 TTS 再测量"
        } else {
            "当前使用云端预设；离线预设须配置本地模型并实测资源"
        }
        .into(),
    })
}
pub async fn measure(
    State(state): State<AppState>,
    body: Result<Json<RuntimeMeasureRequest>, JsonRejection>,
) -> Result<Json<RuntimePresetSnapshot>, ApiError> {
    let Json(request) = body.map_err(|_| conflict("invalid_measurement", "需要音色与测试文本"))?;
    if state.config.llm.mode != "local" {
        return Err(conflict("not_local", "请先配置本地 LLM 预设后重启主服务"));
    }
    let model = state
        .model
        .clone()
        .ok_or_else(|| conflict("llm_unconfigured", "本地模型尚未配置"))?;
    meowlive_domain::speech::SpeechText::new(&request.text)
        .map_err(|_| conflict("invalid_text", "测试文本无效"))?;
    let resources = state.resources.clone();
    let voice = request.voice_id.clone();
    blocking(move || resources.resolve_voice(&voice))
        .await?
        .map_err(|_| conflict("unknown_voice", "测试音色不可用"))?;
    let lease = state.acquire_gpu().await?;
    let owned = state.clone();
    tokio::spawn(async move {
        let _lease = lease;
        *owned.measurement.lock().await = None;
        let baseline = meowlive_adapters::runtime::sample_gpu()
            .await
            .map_err(|e| conflict("gpu_unavailable", e))?;
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let sampler = tokio::spawn(collect_gpu_samples(
            baseline,
            stopped,
            meowlive_adapters::runtime::sample_gpu,
        ));
        let start = Instant::now();
        let result = async {
            let reply = tokio::time::timeout(
                Duration::from_secs(owned.config.llm.timeout_seconds),
                model.decide(DecisionRequest {
                    persona: "你是一位中文主播，请简短回应这条测试弹幕。".into(),
                    topic: "本地联合运行测量".into(),
                    history: vec![],
                    events: vec![LiveEvent {
                        id: "measurement".into(),
                        source: "local".into(),
                        viewer: "测试".into(),
                        occurred_at_ms: 0,
                        kind: EventKind::Chat { text: request.text },
                    }],
                }),
            )
            .await
            .map_err(|_| conflict("measurement_timeout", "本地 LLM 测量超时"))?
            .map_err(|_| conflict("measurement_failed", "本地 LLM 请求失败，请检查模型服务"))?
            .text
            .ok_or_else(|| conflict("measurement_skipped", "模型未生成回复，不能测量语音链路"))?;
            let llm_ms = ms(start.elapsed());
            let tts = Instant::now();
            let audio = tokio::time::timeout(
                Duration::from_secs(owned.config.speech.timeout_seconds + 10),
                owned.synthesizer.synthesize(SynthesisRequest {
                    text: reply.clone(),
                    voice_id: request.voice_id.clone(),
                }),
            )
            .await
            .map_err(|_| conflict("measurement_timeout", "TTS 测量超时"))?
            .map_err(|_| conflict("measurement_failed", "TTS 请求失败，请检查本地推理服务"))?;
            wav(audio)?;
            Ok::<_, ApiError>((reply, llm_ms, ms(tts.elapsed()), ms(start.elapsed())))
        }
        .await;
        let _ = stop.send(());
        let sampled = sampler.await.map_err(|_| internal())?;
        let (reply, llm_ms, tts_ms, total_ms) = result?;
        let (baseline, peak) = sampled.map_err(|error| conflict("gpu_unavailable", error))?;
        let measurement = RuntimeMeasurement {
            measured_at_ms: ms(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()),
            llm_ms,
            tts_ms,
            total_ms,
            gpu_name: baseline.name,
            gpu_total_mib: baseline.total_mib,
            gpu_peak_used_mib: peak,
            voice_id: request.voice_id,
            reply,
            passed: peak <= 5120
                && baseline.total_mib.saturating_sub(peak) >= 512
                && total_ms <= 30000,
        };
        let directory = owned.config.training.directory.clone();
        let record = measurement.clone();
        blocking(move || -> Result<(), ApiError> {
            std::fs::create_dir_all(&directory).map_err(|_| internal())?;
            let path = directory.join(format!("measurement-{}.json", record.measured_at_ms));
            let file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .map_err(|_| internal())?;
            serde_json::to_writer_pretty(file, &record).map_err(|_| internal())
        })
        .await??;
        *owned.measurement.lock().await = Some(measurement);
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|_| internal())??;
    Ok(status(State(state)).await)
}

async fn collect_gpu_samples<S, F>(
    baseline: meowlive_adapters::runtime::GpuSample,
    mut stopped: tokio::sync::oneshot::Receiver<()>,
    mut sample: S,
) -> Result<(meowlive_adapters::runtime::GpuSample, u32), String>
where
    S: FnMut() -> F,
    F: std::future::Future<Output = Result<meowlive_adapters::runtime::GpuSample, String>>,
{
    let mut peak = baseline.used_mib;
    let mut failure = None;
    let mut samples = 0;
    let mut record = |result: Result<meowlive_adapters::runtime::GpuSample, String>| match result {
        Ok(current) if current.name == baseline.name && current.total_mib == baseline.total_mib => {
            peak = peak.max(current.used_mib);
        }
        Ok(current) => {
            failure.get_or_insert_with(|| {
                format!(
                    "GPU 采样设备发生变化：原设备 {}（{} MiB），当前设备 {}（{} MiB）",
                    baseline.name, baseline.total_mib, current.name, current.total_mib,
                )
            });
        }
        Err(error) => {
            failure.get_or_insert(error);
        }
    };
    loop {
        tokio::select! {
            biased;
            _ = &mut stopped => break,
            _ = tokio::time::sleep(Duration::from_millis(250)) => {
                record(sample().await);
                samples += 1;
            }
        }
    }
    record(sample().await);
    if let Some(error) = failure {
        return Err(format!("GPU 测量采样不完整：{error}"));
    }
    if samples == 0 {
        return Err("GPU 测量采样不完整：运行期间未取得采样，无法验证峰值".into());
    }
    Ok((baseline, peak))
}
fn ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use meowlive_adapters::runtime::GpuSample;

    fn gpu(used_mib: u32) -> GpuSample {
        GpuSample {
            name: "Test GPU".into(),
            total_mib: 8192,
            used_mib,
        }
    }

    #[tokio::test]
    async fn intermediate_sampling_failure_survives_a_successful_final_sample() {
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let mut stop = Some(stop);
        let error = collect_gpu_samples(gpu(100), stopped, || {
            let result = if let Some(stop) = stop.take() {
                let _ = stop.send(());
                Err("GPU 0: Failed to initialize NVML".into())
            } else {
                Ok(gpu(200))
            };
            std::future::ready(result)
        })
        .await
        .unwrap_err();
        assert!(error.contains("Failed to initialize NVML"), "{error}");
    }

    #[tokio::test]
    async fn final_sampling_failure_retains_its_reason() {
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let mut stop = Some(stop);
        let error = collect_gpu_samples(gpu(100), stopped, || {
            let result = if let Some(stop) = stop.take() {
                let _ = stop.send(());
                Ok(gpu(300))
            } else {
                Err("GPU 0: Driver/library version mismatch".into())
            };
            std::future::ready(result)
        })
        .await
        .unwrap_err();
        assert!(error.contains("Driver/library version mismatch"), "{error}");
    }

    #[tokio::test]
    async fn successful_sampling_keeps_peak_from_the_running_interval() {
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let mut stop = Some(stop);
        let (baseline, peak) = collect_gpu_samples(gpu(100), stopped, || {
            let current = if let Some(stop) = stop.take() {
                let _ = stop.send(());
                gpu(800)
            } else {
                gpu(200)
            };
            std::future::ready(Ok(current))
        })
        .await
        .unwrap();
        assert_eq!(baseline.used_mib, 100);
        assert_eq!(peak, 800);
    }

    #[tokio::test]
    async fn device_change_and_missing_interval_are_reported_as_incomplete_measurements() {
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let _ = stop.send(());
        let error = collect_gpu_samples(gpu(100), stopped, || std::future::ready(Ok(gpu(200))))
            .await
            .unwrap_err();
        assert!(error.contains("运行期间未取得采样"), "{error}");

        let (stop, stopped) = tokio::sync::oneshot::channel();
        let _ = stop.send(());
        let error = collect_gpu_samples(gpu(100), stopped, || {
            let mut current = gpu(200);
            current.name = "Different GPU".into();
            std::future::ready(Ok(current))
        })
        .await
        .unwrap_err();
        assert!(error.contains("设备发生变化"), "{error}");
        assert!(
            error.contains("Test GPU") && error.contains("Different GPU"),
            "{error}"
        );
    }
}
