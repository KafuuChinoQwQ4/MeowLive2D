//! 资源上传、档案、选择和逐项预览；文件与引擎路径留在服务端。
use super::error::ApiError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Multipart, State, rejection::JsonRejection},
    http::StatusCode,
};
use meowlive_application::resources::ResourceLibrary;
use meowlive_domain::character::{
    CharacterMapping as DomainMapping, CharacterProfile as DomainCharacter,
};
use meowlive_protocol::resources::*;
use std::sync::{Arc, atomic::Ordering};

fn invalid(message: impl ToString) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "invalid_resource",
        message.to_string(),
    )
}
fn json<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(value)| value)
        .map_err(|_| invalid("资源请求格式无效"))
}
async fn work<T: Send + 'static>(
    state: &AppState,
    f: impl FnOnce(&ResourceLibrary) -> Result<T, String> + Send + 'static,
) -> Result<T, ApiError> {
    let resources = state.resources.clone();
    tokio::task::spawn_blocking(move || f(&resources))
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "resource_failed",
                "资源操作未完成",
            )
        })?
        .map_err(invalid)
}
fn character(profile: &DomainCharacter) -> CharacterProfile {
    CharacterProfile {
        id: profile.id.clone(),
        name: profile.name.clone(),
        model_id: profile.model_id.clone(),
        voice_id: profile.voice_id.clone(),
        mouth_parameter: profile.mouth_parameter.clone(),
        mappings: profile
            .mappings
            .iter()
            .map(|m| CharacterMapping {
                intent: m.intent().into(),
                hotkey_id: m.hotkey_id().into(),
                fallback_hotkey_id: m.fallback_hotkey_id().map(str::to_owned),
                validated: m.validated(),
            })
            .collect(),
    }
}
async fn snapshot(state: &AppState) -> Result<ResourceSnapshot, ApiError> {
    let default_voice_available = !state.config.speech.reference_audio.trim().is_empty();
    work(state, move |lib| {
        let catalog = lib.snapshot();
        let voices = catalog
            .voices
            .iter()
            .map(|v| {
                let error = lib.resolve_voice(&v.id).err().map(|e| e.to_string());
                VoiceProfile {
                    id: v.id.clone(),
                    name: v.name.clone(),
                    language: v.language.as_str().into(),
                    reference_text: v.reference_text.clone(),
                    duration_ms: v.duration_ms,
                    sample_rate: v.sample_rate,
                    channels: v.channels,
                    available: error.is_none(),
                    error,
                }
            })
            .collect();
        Ok(ResourceSnapshot {
            voices,
            characters: catalog.characters.iter().map(character).collect(),
            active_voice_id: if catalog.active_voice_id == "default" && !default_voice_available {
                String::new()
            } else {
                catalog.active_voice_id
            },
            active_character_id: catalog.active_character_id,
            default_voice_available,
        })
    })
    .await
}
pub async fn status(State(state): State<AppState>) -> Result<Json<ResourceSnapshot>, ApiError> {
    snapshot(&state).await.map(Json)
}

pub async fn upload(
    State(state): State<AppState>,
    multipart: Result<Multipart, axum::extract::multipart::MultipartRejection>,
) -> Result<Json<ResourceSnapshot>, ApiError> {
    let mut multipart = multipart.map_err(|_| invalid("需要 multipart 参考音频与 metadata"))?;
    let (mut metadata, mut audio) = (None, None);
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| invalid("上传内容无效或超过 2 MiB 限制"))?
    {
        let name = field.name().unwrap_or("").to_owned();
        match name.as_str() {
            "metadata" if metadata.is_none() => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| invalid("音色信息读取失败"))?;
                if bytes.len() > 16 * 1024 {
                    return Err(invalid("音色信息过长"));
                }
                metadata = Some(
                    serde_json::from_slice::<VoiceCreateRequest>(&bytes)
                        .map_err(|_| invalid("音色信息格式无效"))?,
                );
            }
            "audio" if audio.is_none() => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| invalid("参考音频过大或上传失败"))?;
                if bytes.len() > 2 * 1024 * 1024 {
                    return Err(invalid("参考音频不能超过 2 MiB"));
                }
                audio = Some(bytes);
            }
            _ => return Err(invalid("上传字段重复或未知")),
        }
    }
    let metadata = metadata.ok_or_else(|| invalid("缺少音色信息"))?;
    let audio = audio.ok_or_else(|| invalid("缺少参考音频"))?;
    work(&state, move |lib| {
        lib.create_voice(
            &metadata.name,
            &metadata.language,
            &metadata.reference_text,
            &audio,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    })
    .await?;
    snapshot(&state).await.map(Json)
}

pub async fn select_voice(
    State(state): State<AppState>,
    body: Result<Json<ResourceSelection>, JsonRejection>,
) -> Result<Json<ResourceSnapshot>, ApiError> {
    let request = json(body)?;
    if request.id == "default" && state.config.speech.reference_audio.trim().is_empty() {
        return Err(invalid("尚未设置音色，请先上传参考声音"));
    }
    let _edit = state
        .resource_edits
        .try_lock()
        .map_err(|_| invalid("角色资源正在更新"))?;
    work(&state, move |lib| {
        lib.resolve_voice(&request.id).map_err(|e| e.to_string())?;
        lib.select_voice(&request.id).map_err(|e| e.to_string())
    })
    .await?;
    snapshot(&state).await.map(Json)
}

pub async fn save_character(
    State(state): State<AppState>,
    body: Result<Json<CharacterSaveRequest>, JsonRejection>,
) -> Result<Json<ResourceSnapshot>, ApiError> {
    let r = json(body)?;
    let _edit = state
        .resource_edits
        .try_lock()
        .map_err(|_| invalid("角色资源正在更新"))?;
    let update = r.id.is_some();
    let id = r.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mappings = r
        .mappings
        .into_iter()
        .map(|m| DomainMapping::new(m.intent, m.hotkey_id, m.fallback_hotkey_id))
        .collect::<Result<Vec<_>, _>>()
        .map_err(invalid)?;
    let profile = DomainCharacter::new(
        id,
        r.name,
        r.model_id,
        r.voice_id,
        r.mouth_parameter,
        mappings,
    )
    .map_err(invalid)?;
    work(&state, move |lib| {
        if update {
            lib.update_character(profile)
        } else {
            lib.create_character(profile)
        }
        .map_err(|e| e.to_string())
    })
    .await?;
    snapshot(&state).await.map(Json)
}

pub async fn delete_voice(
    State(state): State<AppState>,
    body: Result<Json<ResourceSelection>, JsonRejection>,
) -> Result<Json<ResourceSnapshot>, ApiError> {
    let request = json(body)?;
    let edit = state
        .resource_edits
        .clone()
        .try_lock_owned()
        .map_err(|_| invalid("角色资源正在更新"))?;
    let lease = state.acquire_model_selection().await?;
    let training = state.training.clone();
    work(&state, move |lib| {
        let _edit = edit;
        let _lease = lease;
        if training
            .snapshot()
            .jobs
            .iter()
            .any(|job| job.parameters.voice_id == request.id)
        {
            return Err("该音色仍有关联的训练任务或模型版本，请先在声音训练中删除它们".into());
        }
        lib.delete_voice(&request.id).map_err(|e| e.to_string())
    })
    .await?;
    snapshot(&state).await.map(Json)
}

pub async fn delete_character(
    State(state): State<AppState>,
    body: Result<Json<ResourceSelection>, JsonRejection>,
) -> Result<Json<ResourceSnapshot>, ApiError> {
    let request = json(body)?;
    let edit = state
        .resource_edits
        .clone()
        .try_lock_owned()
        .map_err(|_| invalid("角色资源正在更新"))?;
    let lease = state.acquire_model_selection().await?;
    work(&state, move |lib| {
        let _edit = edit;
        let _lease = lease;
        lib.delete_character(&request.id).map_err(|e| e.to_string())
    })
    .await?;
    snapshot(&state).await.map(Json)
}

struct Changing(Arc<std::sync::atomic::AtomicBool>);
impl Drop for Changing {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
pub async fn select_character(
    State(state): State<AppState>,
    body: Result<Json<ResourceSelection>, JsonRejection>,
) -> Result<Json<ResourceSnapshot>, ApiError> {
    let r = json(body)?;
    let _edit = state
        .resource_edits
        .try_lock()
        .map_err(|_| invalid("角色资源正在更新"))?;
    let selected = r.id.clone();
    let profile = work(&state, move |lib| {
        lib.snapshot()
            .characters
            .into_iter()
            .find(|c| c.id == selected)
            .ok_or_else(|| "角色不存在".into())
    })
    .await?;
    let voice = profile.voice_id.clone();
    if voice == "default" && state.config.speech.reference_audio.trim().is_empty() {
        return Err(invalid("角色尚未设置可用音色，请先上传参考声音并更新角色"));
    }
    work(&state, move |lib| {
        lib.resolve_voice(&voice)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await?;
    {
        let inner = state.inner.lock().await;
        state.require_gpu_idle()?;
        if inner.queue.tasks().any(|t| !t.status.is_terminal()) {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "speech_busy",
                "请停止播报后切换角色",
            ));
        }
        state.resource_changing.store(true, Ordering::Release);
    }
    let _changing = Changing(state.resource_changing.clone());
    state.pause_agent().await;
    let result = state
        .desktop_resource(DesktopResourceOperation::LoadModel {
            model_id: profile.model_id.clone(),
            mouth_parameter: profile.mouth_parameter.clone(),
        })
        .await?;
    match result {
        DesktopResourceResult::ModelLoaded { model_id } if model_id == profile.model_id => {}
        other => return Err(desktop_error(other)),
    }
    work(&state, move |lib| {
        lib.select_character(&r.id).map_err(|e| e.to_string())
    })
    .await?;
    snapshot(&state).await.map(Json)
}

pub async fn preview(
    State(state): State<AppState>,
    body: Result<Json<CharacterPreviewRequest>, JsonRejection>,
) -> Result<Json<ResourceSnapshot>, ApiError> {
    let r = json(body)?;
    let _edit = state
        .resource_edits
        .try_lock()
        .map_err(|_| invalid("角色资源正在更新"))?;
    let selected = r.character_id.clone();
    let profile = work(&state, move |lib| {
        lib.snapshot()
            .characters
            .into_iter()
            .find(|c| c.id == selected)
            .ok_or_else(|| "角色不存在".into())
    })
    .await?;
    let mapping = profile
        .mappings
        .iter()
        .find(|m| m.intent() == r.intent)
        .ok_or_else(|| invalid("该角色未配置此能力"))?;
    let operation = DesktopResourceOperation::TriggerHotkey {
        model_id: profile.model_id.clone(),
        hotkey_id: mapping.hotkey_id().into(),
        fallback_hotkey_id: mapping.fallback_hotkey_id().map(str::to_owned),
    };
    let result = state.desktop_resource(operation).await?;
    match result {
        DesktopResourceResult::HotkeyTriggered { hotkey_id }
            if hotkey_id == mapping.hotkey_id()
                || mapping.fallback_hotkey_id() == Some(hotkey_id.as_str()) => {}
        other => return Err(desktop_error(other)),
    }
    work(&state, move |lib| {
        lib.mark_mapping_validated_if_current(&profile, &r.intent)
            .map_err(|e| e.to_string())
    })
    .await?;
    snapshot(&state).await.map(Json)
}

fn desktop_error(result: DesktopResourceResult) -> ApiError {
    let message: String = match result {
        DesktopResourceResult::Error { message, .. } => message.chars().take(1000).collect(),
        _ => "桌面返回的资源结果与请求不符".into(),
    };
    ApiError::new(StatusCode::BAD_GATEWAY, "desktop_resource_failed", message)
}
pub async fn desktop(
    State(state): State<AppState>,
    body: Result<Json<DesktopResourceOperation>, JsonRejection>,
) -> Result<Json<DesktopResourceResult>, ApiError> {
    let operation = json(body)?;
    if let DesktopResourceOperation::DeleteImportedModel { id } = operation {
        if !valid_imported_id(&id) {
            return Err(invalid("模型标识无效"));
        }
        let edit = state
            .resource_edits
            .clone()
            .try_lock_owned()
            .map_err(|_| invalid("角色资源正在更新"))?;
        let lease = state.acquire_model_selection().await?;
        // A disconnected HTTP caller must not release the deletion fence early.
        return tokio::spawn(async move {
            let _edit = edit;
            let _lease = lease;
            let listed = state.desktop_resource(DesktopResourceOperation::ListImportedModels).await?;
            let DesktopResourceResult::ImportedModels { models } = listed else {
                return Err(desktop_error(listed));
            };
            if !valid_imported_models(&models) { return Err(invalid("桌面返回的模型列表无效")); }
            let model = models.into_iter().find(|model| model.id == id)
                .ok_or_else(|| invalid("模型不存在，请刷新列表"))?;
            work(&state, move |lib| {
                    let characters = lib.snapshot().characters;
                    if model.model_id.is_none() && !characters.is_empty() {
                        return Err("无法确认该模型与角色的对应关系，请先在 VTS 加载一次以生成模型标识，再卸载并刷新列表".into());
                    }
                    if characters.iter().any(|role| model.model_id.as_deref() == Some(role.model_id.as_str())) {
                        Err("该模型仍被角色配置使用，请先编辑或删除对应角色配置".into())
                    } else { Ok(()) }
                }).await?;
            let result = state.desktop_resource(DesktopResourceOperation::DeleteImportedModel { id: id.clone() }).await?;
            match &result {
                DesktopResourceResult::ModelDeleted { id: received, .. } if received == &id => Ok(Json(result)),
                DesktopResourceResult::Error { code, message } if bounded(code, 128) && bounded(message, 1000) => Ok(Json(result)),
                _ => Err(desktop_error(result)),
            }
        }).await.map_err(|_| invalid("模型删除任务中断"))?;
    }
    match &operation {
        DesktopResourceOperation::ListModels
        | DesktopResourceOperation::ImportModel
        | DesktopResourceOperation::ListImportedModels => {}
        DesktopResourceOperation::ListHotkeys { model_id }
            if !model_id.trim().is_empty() && model_id.len() <= 128 => {}
        _ => return Err(invalid("请通过角色选择或预览操作使用此能力")),
    }
    let result = state.desktop_resource(operation.clone()).await?;
    let valid = match (&operation, &result) {
        (
            DesktopResourceOperation::ListImportedModels,
            DesktopResourceResult::ImportedModels { models },
        ) => valid_imported_models(models),
        (DesktopResourceOperation::ListModels, DesktopResourceResult::Models { models }) => {
            models.len() <= 256
                && models
                    .iter()
                    .all(|m| bounded(&m.id, 128) && bounded(&m.name, 256))
        }
        (
            DesktopResourceOperation::ListHotkeys { model_id },
            DesktopResourceResult::Hotkeys {
                model_id: received,
                hotkeys,
            },
        ) => {
            model_id == received
                && hotkeys.len() <= 256
                && hotkeys
                    .iter()
                    .all(|h| bounded(&h.id, 128) && bounded(&h.name, 256))
        }
        (
            DesktopResourceOperation::ImportModel,
            DesktopResourceResult::ModelImported {
                model_name,
                model_file,
                files,
                bytes,
                ..
            },
        ) => bounded(model_name, 256) && bounded(model_file, 256) && *files > 0 && *bytes > 0,
        (_, DesktopResourceResult::Error { code, message }) => {
            bounded(code, 128) && bounded(message, 1000)
        }
        _ => false,
    };
    if !valid {
        return Err(desktop_error(result));
    }
    Ok(Json(result))
}
fn bounded(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max
}

fn valid_imported_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_imported_models(models: &[ImportedModel]) -> bool {
    let mut ids = std::collections::HashSet::new();
    models.len() <= 256
        && models.iter().all(|model| {
            valid_imported_id(&model.id)
                && ids.insert(&model.id)
                && bounded(&model.name, 256)
                && model.model_id.as_deref().is_none_or(|id| bounded(id, 128))
        })
}
