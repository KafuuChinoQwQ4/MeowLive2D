//! 成对权重加载和合成共享一个锁；即使调用方取消也完成有界引擎事务。
use meowlive_application::{
    ports::speech::{
        PcmAudio, SpeechSynthesizer, SynthesisError, SynthesisFuture, SynthesisRequest,
    },
    training::TrainingManager,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub struct ModelSynthesizer {
    inner: Arc<dyn SpeechSynthesizer>,
    training: Arc<TrainingManager>,
    engine: Arc<Engine>,
}
struct Engine {
    client: reqwest::Client,
    base: reqwest::Url,
    default_gpt: String,
    default_sovits: String,
    busy: AtomicBool,
    poisoned: AtomicBool,
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ModelRuntimeStatus {
    pub supported: bool,
    pub state: String,
    pub message: String,
}
impl ModelSynthesizer {
    pub fn new(
        inner: Arc<dyn SpeechSynthesizer>,
        training: Arc<TrainingManager>,
        base: &str,
        default_gpt: String,
        default_sovits: String,
        timeout: Duration,
    ) -> Result<Self, SynthesisError> {
        let base = reqwest::Url::parse(base).map_err(|_| error("受管推理地址无效"))?;
        let local = base.host_str().is_some_and(|host| {
            host.trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        });
        if !local
            || base.scheme() != "http"
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || default_gpt.is_empty()
            || default_sovits.is_empty()
        {
            return Err(error(
                "权重管理仅支持显式配置的本机受管推理实例及默认权重对",
            ));
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .build()
            .map_err(|_| error("无法初始化权重管理"))?;
        Ok(Self {
            inner,
            training,
            engine: Arc::new(Engine {
                client,
                base,
                default_gpt,
                default_sovits,
                busy: AtomicBool::new(false),
                poisoned: AtomicBool::new(false),
            }),
        })
    }
    pub fn is_busy(&self) -> bool {
        self.engine.busy.load(Ordering::Acquire)
    }
    pub async fn model_status(&self) -> Result<ModelRuntimeStatus, SynthesisError> {
        self.engine.runtime_request(None).await
    }
    pub async fn set_models_enabled(
        &self,
        enabled: bool,
    ) -> Result<ModelRuntimeStatus, SynthesisError> {
        if self
            .engine
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(error("模型正在合成或切换，请稍后重试"));
        }
        let busy = Busy(self.engine.clone());
        let engine = self.engine.clone();
        tokio::spawn(async move {
            let _busy = busy;
            let result = engine.runtime_request(Some(enabled)).await?;
            if result.supported && result.state == if enabled { "loaded" } else { "unloaded" } {
                engine.poisoned.store(false, Ordering::Release);
                Ok(result)
            } else {
                Err(error("引擎未确认模型启停，请刷新状态后重试"))
            }
        })
        .await
        .map_err(|_| error("模型启停操作未完成"))?
    }
    pub async fn audition(&self, version: &str, text: String) -> Result<PcmAudio, SynthesisError> {
        self.generate(None, Some(version.to_owned()), text).await
    }
    async fn generate(
        &self,
        voice: Option<String>,
        version: Option<String>,
        text: String,
    ) -> Result<PcmAudio, SynthesisError> {
        if self.engine.poisoned.load(Ordering::Acquire) {
            return Err(error("权重状态不确定，请重启受管推理与主服务"));
        }
        if self
            .engine
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(error("上一次模型合成尚未结束，请稍后重试"));
        }
        let busy = Busy(self.engine.clone());
        let training = self.training.clone();
        let inner = self.inner.clone();
        let engine = self.engine.clone();
        // Own the transaction: dropping the HTTP/speech future never drops a half-applied pair.
        tokio::spawn(async move {
            let _busy = busy;
            let status = engine.runtime_request(None).await?;
            if status.supported && status.state != "loaded" {
                return Err(error("语音模型未就绪，请先在训练音色页面启用模型"));
            }
            let (voice_id, pair) = tokio::task::spawn_blocking(move || {
                if let Some(id) = version {
                    let pair = training
                        .resolve_version(&id)
                        .map_err(|e| error(e.to_string()))?;
                    let voice_id = training
                        .snapshot()
                        .jobs
                        .into_iter()
                        .find(|j| j.id == id)
                        .ok_or_else(|| error("模型版本不存在"))?
                        .parameters
                        .voice_id;
                    Ok::<_, SynthesisError>((voice_id, Some(pair)))
                } else {
                    let voice = voice.unwrap_or_else(|| "default".into());
                    let pair = training
                        .resolve_active_pair(&voice)
                        .map_err(|e| error(e.to_string()))?;
                    Ok((voice, pair))
                }
            })
            .await
            .map_err(|_| error("权重版本查询未完成"))??;
            let (gpt, sovits) = pair
                .as_ref()
                .map(|p| {
                    (
                        p.gpt.to_str().unwrap_or(""),
                        p.sovits.to_str().unwrap_or(""),
                    )
                })
                .unwrap_or((&engine.default_gpt, &engine.default_sovits));
            if engine.set_pair(gpt, sovits).await.is_err() {
                let restored = engine
                    .set_pair(&engine.default_gpt, &engine.default_sovits)
                    .await
                    .is_ok();
                if !restored {
                    engine.poisoned.store(true, Ordering::Release);
                }
                return Err(error(if restored {
                    "权重加载失败，已恢复默认权重；请检查训练版本"
                } else {
                    "权重加载与恢复失败，请重启受管推理实例"
                }));
            }
            inner.synthesize(SynthesisRequest { text, voice_id }).await
        })
        .await
        .map_err(|_| error("权重合成任务未完成"))?
    }
}
impl SpeechSynthesizer for ModelSynthesizer {
    fn synthesize(&self, request: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(self.generate(Some(request.voice_id), None, request.text))
    }

    fn owns_synthesis_lifetime(&self) -> bool {
        true
    }
}
impl Engine {
    async fn runtime_request(
        &self,
        enabled: Option<bool>,
    ) -> Result<ModelRuntimeStatus, SynthesisError> {
        runtime_request(&self.client, &self.base, enabled).await
    }
    async fn set_pair(&self, gpt: &str, sovits: &str) -> Result<(), SynthesisError> {
        for (route, path) in [("set_gpt_weights", gpt), ("set_sovits_weights", sovits)] {
            let mut url = self.base.clone();
            url.set_path(&format!(
                "{}/{route}",
                self.base.path().trim_end_matches('/')
            ));
            url.query_pairs_mut().append_pair("weights_path", path);
            let mut response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|_| error("无法连接权重管理接口"))?;
            if !response.status().is_success() {
                return Err(error("引擎拒绝权重切换"));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| error("权重切换响应无效"))?
            {
                if bytes.len() + chunk.len() > 1024 {
                    return Err(error("权重切换响应超限"));
                }
                bytes.extend_from_slice(&chunk);
            }
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| error("权重切换响应无效"))?;
            if value.get("message").and_then(|v| v.as_str()) != Some("success") {
                return Err(error("引擎未确认权重切换"));
            }
        }
        Ok(())
    }
}
/// Control only the project-specific lifecycle API on a local TTS service.
/// This also supports reference-only setups without training/weight management.
pub async fn request_model_runtime(
    base: &str,
    enabled: Option<bool>,
) -> Result<ModelRuntimeStatus, SynthesisError> {
    let base = reqwest::Url::parse(base).map_err(|_| error("受管推理地址无效"))?;
    if base.scheme() != "http"
        || !base.host_str().is_some_and(|host| {
            host.trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        })
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err(error("模型开关仅支持项目受管的本机 TTS"));
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| error("无法初始化模型开关"))?;
    runtime_request(&client, &base, enabled).await
}

async fn runtime_request(
    client: &reqwest::Client,
    base: &reqwest::Url,
    enabled: Option<bool>,
) -> Result<ModelRuntimeStatus, SynthesisError> {
    let mut url = base.clone();
    url.set_path(&format!(
        "{}/meowlive/models",
        base.path().trim_end_matches('/')
    ));
    let request = match enabled {
        Some(enabled) => client
            .post(url)
            .timeout(Duration::from_secs(300))
            .json(&serde_json::json!({"enabled":enabled})),
        None => client.get(url).timeout(Duration::from_secs(2)),
    };
    let mut response = request
        .send()
        .await
        .map_err(|_| error("无法连接 TTS 服务，请先启动 TTS"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND && enabled.is_none() {
        return Ok(ModelRuntimeStatus {
            supported: false,
            state: "unsupported".into(),
            message: "当前 TTS 不支持独立模型开关，请重启项目受管 TTS".into(),
        });
    }
    if !response.status().is_success() {
        return Err(error("模型操作失败，请检查 TTS 日志并刷新模型状态"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| error("模型状态响应无效"))?
    {
        if bytes.len() + chunk.len() > 4096 {
            return Err(error("模型状态响应超限"));
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: ModelRuntimeStatus =
        serde_json::from_slice(&bytes).map_err(|_| error("模型状态响应无效"))?;
    if !value.supported
        || !matches!(
            value.state.as_str(),
            "unloaded" | "loading" | "loaded" | "unloading" | "failed"
        )
        || value.message.chars().count() > 300
    {
        return Err(error("模型状态响应无效"));
    }
    Ok(value)
}

fn error(message: impl Into<String>) -> SynthesisError {
    SynthesisError::new(message)
}

struct Busy(Arc<Engine>);
impl Drop for Busy {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::Release);
    }
}
