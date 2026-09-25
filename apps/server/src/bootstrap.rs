//! 读取配置、注入语音适配器并管理主服务与后台 worker 生命周期。
pub use crate::live::bootstrap::build_live_source;
use crate::{
    agent::run_agent,
    config::{AppConfig, LlmConfig},
    state::AppState,
    transport::http::router,
    worker::run_worker,
};
use meowlive_adapters::llm::multi_provider::{LlmConfig as AdapterConfig, MultiProvider};
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
    if config.viewers.receipt_directory.is_relative() {
        config.viewers.receipt_directory = config_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(&config.viewers.receipt_directory);
    }
    config.training.resolve(config_path)?;
    let auth = Arc::new(crate::auth::AdminAuth::from_config(
        &config.auth,
        config_path,
    )?);
    let viewer_store = if config.viewers.enabled {
        let url = std::env::var(&config.viewers.database_url_env)
            .map_err(|_| "观众数据库环境变量未设置")?;
        Some(Arc::new(
            meowlive_adapters::storage::postgres::PostgresViewerEventStore::connect_with_options(
                &url,
                meowlive_adapters::storage::postgres::PostgresViewerStoreOptions {
                    calendar_offset_minutes: config.viewers.calendar_offset_minutes,
                    ..Default::default()
                },
            )
            .await
            .map_err(|_| "观众数据库连接或迁移失败")?,
        ))
    } else {
        None
    };
    let journal = if config.viewers.enabled {
        Some(Arc::new(
            meowlive_adapters::storage::receipt_journal::FileReceiptJournal::open(
                &config.viewers.receipt_directory,
                4096,
            )
            .map_err(|_| "完成回执暂存目录无法打开")?,
        ))
    } else {
        None
    };
    let training = build_training(&config.training)?;
    let resources = build_resources(&config.resources)?;
    if config.speech.reference_audio.trim().is_empty()
        && resources.snapshot().active_voice_id == "default"
    {
        resources
            .clear_voice_selection_if_current("default")
            .map_err(|error| error.to_string())?;
    }
    let model = match build_model(&config.llm) {
        Ok(model) => model,
        Err(_) => {
            eprintln!(
                "LLM 尚未就绪；请在控制面板的 LLM 接入页检查配置、测试并保存，再重启主服务。"
            );
            None
        }
    };
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
    state.llm_runtime = Arc::new(crate::llm_runtime::RuntimeStore::open(
        state.config.resources.directory.join("agent-runtime"),
    )?);
    state.agent_observability = Arc::new(crate::agent_observability::AgentTraceStore::open(
        state
            .config
            .resources
            .directory
            .join("agent-runtime/traces"),
    )?);
    state.live_settings = Arc::new(crate::live_settings::LiveSettingsStore::new(
        crate::live_settings::settings_path(config_path),
    ));
    state.agent_settings = Arc::new(crate::agent_settings::AgentSettingsStore::new(
        crate::agent_settings::settings_path(config_path),
    ));
    state.llm_settings = Arc::new(crate::llm_settings::LlmSettingsStore::new(
        state.config.llm.clone(),
        Some(crate::llm_settings::settings_path(config_path)),
    ));
    state.auth = auth;
    state.receipt_journal =
        journal.map(|j| j as Arc<dyn meowlive_application::ports::receipt_journal::ReceiptJournal>);
    state.viewer_merge_store = viewer_store
        .clone()
        .map(|store| store as Arc<dyn meowlive_application::ports::viewer_merge::ViewerMergeStore>);
    state.relationship_store = viewer_store.clone().map(|store| {
        store as Arc<dyn meowlive_application::ports::relationships::RelationshipStore>
    });
    state.memory_store = viewer_store
        .clone()
        .map(|store| store as Arc<dyn meowlive_application::ports::memory_store::MemoryStore>);
    state.companionship_store = viewer_store.clone().map(|store| {
        store as Arc<dyn meowlive_application::ports::companionship::CompanionshipStore>
    });
    state.viewer_store = viewer_store
        .map(|store| store as Arc<dyn meowlive_application::ports::viewers::ViewerEventStore>);
    state.resources = resources;
    state.training = training;
    state.model_synthesizer = model_synthesizer;
    if state.config.memory.enabled {
        state.memory_extractor = Some(Arc::new(memory_adapter(&state.config.memory, false)?));
        if !state.config.memory.embedding_endpoint.is_empty() {
            state.memory_embedder = Some(Arc::new(memory_adapter(&state.config.memory, true)?));
        }
    }
    let worker = tokio::spawn(run_worker(state.clone()));
    let agent = tokio::spawn(run_agent(state.clone()));
    let mut receipts = tokio::spawn(crate::companionship::run_receipts(state.clone()));
    let graph = tokio::spawn(crate::graph::run_graph(state.clone()));
    let expiry = tokio::spawn(crate::memory::run_knowledge_expiry(state.clone()));
    let memory = tokio::spawn(crate::memory::run_memory(state.clone()));
    let shutdown_state = state.clone();
    let result = axum::serve(listener, router(state.clone()))
        .with_graceful_shutdown(async move {
            let parent_closed = async {
                if std::env::var("MEOWLIVE_DESKTOP_PARENT").as_deref() != Ok("1") {
                    std::future::pending::<()>().await;
                }
                let (closed, receiver) = tokio::sync::oneshot::channel();
                std::thread::spawn(move || {
                    use std::io::Read;
                    let mut input = std::io::stdin().lock();
                    let mut buffer = [0; 256];
                    while matches!(input.read(&mut buffer), Ok(count) if count > 0) {}
                    let _ = closed.send(());
                });
                let _ = receiver.await;
            };
            let signal = async {
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
            };
            tokio::select! { _ = parent_closed => {}, _ = signal => {} }
            shutdown_state.shutdown().await;
        })
        .await
        .map_err(|e| format!("服务退出：{e}"));
    state.shutdown().await;
    worker.abort();
    agent.abort();
    if tokio::time::timeout(Duration::from_secs(4), &mut receipts)
        .await
        .is_err()
    {
        receipts.abort();
        let _ = receipts.await;
    }
    memory.abort();
    expiry.abort();
    graph.abort();
    let _ = worker.await;
    let _ = agent.await;
    let _ = memory.await;
    let _ = expiry.await;
    let _ = graph.await;
    result
}

pub fn build_model(config: &LlmConfig) -> Result<Option<Arc<dyn LanguageModel>>, String> {
    config.validate()?;
    if !config.is_configured() {
        return Ok(None);
    }
    let api_key = if let Some(key) = &config.api_key {
        Some(key.clone())
    } else if config.api_key_env.is_empty() {
        None
    } else {
        let key =
            std::env::var(&config.api_key_env).map_err(|_| "LLM 密钥环境变量未设置或编码无效")?;
        if key.trim().is_empty() {
            return Err("LLM 密钥环境变量为空".into());
        }
        Some(key)
    };
    let model = MultiProvider::new(
        AdapterConfig {
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            api_key,
            timeout: Duration::from_secs(config.timeout_seconds),
            max_response_bytes: config.max_response_bytes,
            max_tokens: config.max_tokens,
            json_mode: config.json_mode,
        },
        config.api_format.parse().map_err(|_| "LLM API 格式无效")?,
    )
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
    .and_then(|engine| {
        engine.with_transcription(config.directory.clone(), config.asr_model.clone())
    })
    .map_err(|e| e.to_string())?;
    TrainingManager::open(Arc::new(store), Arc::new(engine))
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

fn memory_adapter(
    config: &crate::config::MemoryConfig,
    embedding: bool,
) -> Result<meowlive_adapters::memory::HttpMemoryAdapter, String> {
    let (endpoint, model, key_env) = if embedding {
        (
            &config.embedding_endpoint,
            &config.embedding_model,
            &config.embedding_api_key_env,
        )
    } else {
        (&config.endpoint, &config.model, &config.api_key_env)
    };
    let api_key = if key_env.is_empty() {
        String::new()
    } else {
        std::env::var(key_env).map_err(|_| "记忆服务私有密钥未设置")?
    };
    meowlive_adapters::memory::HttpMemoryAdapter::new(meowlive_adapters::memory::AdapterConfig {
        endpoint: endpoint.clone(),
        model: model.clone(),
        api_key,
        dimensions: config.embedding_dimensions,
        timeout_ms: config.timeout_ms,
    })
    .map_err(|_| "记忆服务配置无效".into())
}
