//! 读取配置、注入语音适配器并管理主服务与后台 worker 生命周期。
pub use crate::live::bootstrap::build_live_source;
use crate::{
    agent::run_agent,
    config::{AppConfig, LlmConfig},
    state::AppState,
    transport::http::router,
    worker::run_worker,
};
use meowlive_adapters::llm::openai_compatible::{LlmConfig as AdapterConfig, OpenAiCompatible};
use meowlive_adapters::speech::gpt_sovits::{GptSovits, GptSovitsConfig};
use meowlive_application::ports::llm::LanguageModel;
use meowlive_application::ports::speech::{
    SpeechSynthesizer, SynthesisError, SynthesisFuture, SynthesisRequest,
};
use std::{path::Path, sync::Arc, time::Duration};

struct UnconfiguredSpeech;
impl SpeechSynthesizer for UnconfiguredSpeech {
    fn synthesize(&self, _: SynthesisRequest) -> SynthesisFuture<'_> {
        Box::pin(async {
            Err(SynthesisError::new(
                "请先上传并选择音色，或配置 speech.reference_audio 和参考文本后重启",
            ))
        })
    }
}

pub async fn run(config_path: &Path) -> Result<(), String> {
    let mut config = AppConfig::load(config_path)?;
    config.resources.resolve(config_path)?;
    config.training.resolve(config_path)?;
    let training = build_training(&config.training)?;
    let resources = build_resources(&config.resources)?;
    let model = build_model(&config.llm)?;
    let live_source = build_live_source(&config.live)?;
    let speech = &config.speech;
    let synthesizer: Arc<dyn SpeechSynthesizer> = if speech.reference_audio.trim().is_empty() {
        eprintln!("语音参考素材尚未配置；服务可启动，播报任务会报告配置错误。");
        Arc::new(UnconfiguredSpeech)
    } else {
        Arc::new(
            GptSovits::new(GptSovitsConfig {
                base_url: speech.base_url.clone(),
                reference_audio: speech.reference_audio.clone(),
                prompt_text: speech.prompt_text.clone(),
                prompt_language: speech.prompt_language.clone(),
                text_language: speech.text_language.clone(),
                timeout: Duration::from_secs(speech.timeout_seconds),
                max_audio_bytes: speech.max_audio_bytes,
            })
            .map_err(|e| e.to_string())?,
        )
    };
    let synthesizer = Arc::new(
        meowlive_adapters::speech::resource_synthesizer::ResourceSynthesizer::new(
            synthesizer,
            resources.clone(),
            meowlive_adapters::speech::resource_synthesizer::ResourceSynthesizerConfig {
                base_url: speech.base_url.clone(),
                timeout: Duration::from_secs(speech.timeout_seconds),
                max_audio_bytes: speech.max_audio_bytes,
            },
        )
        .map_err(|e| e.to_string())?,
    );
    let model_synthesizer = if config.training.managed_inference {
        Some(Arc::new(
            meowlive_adapters::speech::model_synthesizer::ModelSynthesizer::new(
                synthesizer.clone(),
                training.clone(),
                &config.speech.base_url,
                config
                    .training
                    .default_gpt_weights
                    .to_string_lossy()
                    .into_owned(),
                config
                    .training
                    .default_sovits_weights
                    .to_string_lossy()
                    .into_owned(),
                Duration::from_secs(30),
            )
            .map_err(|e| e.to_string())?,
        ))
    } else {
        None
    };
    let synthesizer: Arc<dyn SpeechSynthesizer> = model_synthesizer
        .as_ref()
        .map(|s| s.clone() as Arc<dyn SpeechSynthesizer>)
        .unwrap_or(synthesizer);
    let listener = tokio::net::TcpListener::bind(&config.server.listen_address)
        .await
        .map_err(|e| format!("监听失败：{e}"))?;
    eprintln!(
        "MeowLive2D 主服务：http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let mut state = AppState::with_services(config, synthesizer, model, live_source);
    state.resources = resources;
    state.training = training;
    state.model_synthesizer = model_synthesizer;
    let worker = tokio::spawn(run_worker(state.clone()));
    let agent = tokio::spawn(run_agent(state.clone()));
    let shutdown_state = state.clone();
    let result = axum::serve(listener, router(state.clone()))
        .with_graceful_shutdown(async move {
            #[cfg(unix)]
            {
                if let Ok(mut terminate) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    tokio::select! {_=tokio::signal::ctrl_c()=>{},_=terminate.recv()=>{}}
                } else {
                    let _ = tokio::signal::ctrl_c().await;
                }
            }
            #[cfg(not(unix))]
            let _ = tokio::signal::ctrl_c().await;
            shutdown_state.shutdown().await;
        })
        .await
        .map_err(|e| format!("服务退出：{e}"));
    state.shutdown().await;
    worker.abort();
    agent.abort();
    let _ = worker.await;
    let _ = agent.await;
    result
}

pub fn build_model(config: &LlmConfig) -> Result<Option<Arc<dyn LanguageModel>>, String> {
    config.validate()?;
    if !config.is_configured() {
        return Ok(None);
    }
    let api_key = if config.api_key_env.is_empty() {
        None
    } else {
        let key =
            std::env::var(&config.api_key_env).map_err(|_| "LLM 密钥环境变量未设置或编码无效")?;
        if key.trim().is_empty() {
            return Err("LLM 密钥环境变量为空".into());
        }
        Some(key)
    };
    let model = OpenAiCompatible::new(AdapterConfig {
        base_url: config.base_url.clone(),
        model: config.model.clone(),
        api_key,
        timeout: Duration::from_secs(config.timeout_seconds),
        max_response_bytes: config.max_response_bytes,
        max_tokens: config.max_tokens,
        json_mode: config.json_mode,
    })
    .map_err(|error| error.message)?;
    Ok(Some(Arc::new(model)))
}

pub fn build_resources(
    config: &crate::config::ResourcesConfig,
) -> Result<Arc<meowlive_application::resources::ResourceLibrary>, String> {
    use meowlive_adapters::storage::resources::{FileResourceStore, FileResourceStoreConfig};
    std::fs::create_dir_all(&config.directory).map_err(|_| "无法建立音色资源目录")?;
    let root = config
        .directory
        .canonicalize()
        .map_err(|_| "无法解析音色资源目录")?;
    let engine_root = if config.engine_directory.is_empty() {
        root.to_string_lossy().into_owned()
    } else {
        config.engine_directory.clone()
    };
    let store = FileResourceStore::new(FileResourceStoreConfig {
        storage_root: root,
        engine_root,
    })
    .map_err(|e| e.to_string())?;
    meowlive_application::resources::ResourceLibrary::open(Arc::new(store))
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

pub fn build_training(
    config: &crate::config::TrainingConfig,
) -> Result<Arc<meowlive_application::training::TrainingManager>, String> {
    use meowlive_adapters::training::{
        FileTrainingStore, ProcessTrainingConfig, ProcessTrainingEngine,
    };
    use meowlive_application::training::TrainingManager;
    if !config.enabled {
        return Ok(Arc::new(TrainingManager::disabled()));
    }
    let runner = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/train-gpt-sovits.py");
    let store = FileTrainingStore::open(&config.directory).map_err(|e| e.to_string())?;
    let engine = ProcessTrainingEngine::new(ProcessTrainingConfig {
        python: config.python.clone(),
        runner,
        timeout: Duration::from_secs(config.timeout_seconds),
    })
    .and_then(|engine| engine.with_engine_root(config.engine_root.clone()))
    .map_err(|e| e.to_string())?;
    TrainingManager::open(Arc::new(store), Arc::new(engine))
        .map(Arc::new)
        .map_err(|e| e.to_string())
}
