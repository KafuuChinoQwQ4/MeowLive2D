//! 已认证的 VTS 模型与热键资源操作；导入仍由本地文件系统模块完成。
use super::{
    VtsConfig,
    client::{Client, Error as ClientError},
    token,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fmt, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtsModelInfo {
    pub id: String,
    pub name: String,
    pub loaded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtsHotkeyInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceError {
    InvalidConfiguration(String),
    AuthorizationRequired,
    ConnectionFailed,
    RequestTimeout,
    Api(i64),
    InvalidResponse,
    CurrentModelChanged,
    UnexpectedModel,
    UnexpectedHotkey,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => formatter.write_str(message),
            Self::AuthorizationRequired => formatter.write_str("VTS 尚未授权或授权已失效"),
            Self::ConnectionFailed => formatter.write_str("VTS 连接失败"),
            Self::RequestTimeout => formatter.write_str("VTS 请求超时"),
            Self::Api(code) => write!(formatter, "VTS API 请求被拒绝（错误码 {code}）"),
            Self::InvalidResponse => formatter.write_str("VTS 资源响应格式无效"),
            Self::CurrentModelChanged => formatter.write_str("VTS 当前模型已切换，已取消预览"),
            Self::UnexpectedModel => formatter.write_str("VTS 加载响应的模型不匹配"),
            Self::UnexpectedHotkey => formatter.write_str("VTS 热键响应不匹配"),
        }
    }
}

impl std::error::Error for ResourceError {}

pub struct VtsResources {
    client: Client,
    deadline: Duration,
}

impl VtsResources {
    pub async fn connect(config: &VtsConfig) -> Result<Self, ResourceError> {
        config
            .validate()
            .map_err(ResourceError::InvalidConfiguration)?;
        let credential = token::load(config.token_path.clone())
            .await
            .map_err(ResourceError::InvalidConfiguration)?
            .ok_or(ResourceError::AuthorizationRequired)?;
        let deadline = Duration::from_millis(config.request_timeout_ms);
        let mut client = Client::connect(&config.websocket_url, deadline)
            .await
            .map_err(map_client_error)?;
        let response = client
            .request(
                "AuthenticationRequest",
                "AuthenticationResponse",
                json!({"pluginName":"MeowLive2D","pluginDeveloper":"MeowLive2D",
                    "authenticationToken":credential}),
                deadline,
            )
            .await
            .map_err(map_client_error)?;
        match response["authenticated"].as_bool() {
            Some(true) => Ok(Self { client, deadline }),
            Some(false) => Err(ResourceError::AuthorizationRequired),
            None => Err(ResourceError::InvalidResponse),
        }
    }

    pub async fn available_models(&mut self) -> Result<Vec<VtsModelInfo>, ResourceError> {
        let data = self
            .request(
                "AvailableModelsRequest",
                "AvailableModelsResponse",
                json!({}),
            )
            .await?;
        let response: AvailableModels =
            serde_json::from_value(data).map_err(|_| ResourceError::InvalidResponse)?;
        if response.number_of_models != response.available_models.len() as u64 {
            return Err(ResourceError::InvalidResponse);
        }
        response
            .available_models
            .into_iter()
            .map(|model| {
                valid_text(&model.model_id)?;
                valid_text(&model.model_name)?;
                Ok(VtsModelInfo {
                    id: model.model_id,
                    name: model.model_name,
                    loaded: model.model_loaded,
                })
            })
            .collect()
    }

    pub async fn load_model(&mut self, model_id: &str) -> Result<(), ResourceError> {
        valid_text(model_id)?;
        let data = self
            .request(
                "ModelLoadRequest",
                "ModelLoadResponse",
                json!({"modelID":model_id}),
            )
            .await?;
        if data["modelID"].as_str() != Some(model_id) {
            return Err(ResourceError::UnexpectedModel);
        }
        Ok(())
    }

    pub async fn hotkeys(
        &mut self,
        model_id: Option<&str>,
    ) -> Result<Vec<VtsHotkeyInfo>, ResourceError> {
        let data = match model_id {
            Some(model_id) => {
                valid_text(model_id)?;
                json!({"modelID":model_id})
            }
            None => json!({}),
        };
        let value = self
            .request(
                "HotkeysInCurrentModelRequest",
                "HotkeysInCurrentModelResponse",
                data,
            )
            .await?;
        let response: AvailableHotkeys =
            serde_json::from_value(value).map_err(|_| ResourceError::InvalidResponse)?;
        if let Some(expected) = model_id
            && response.model_id != expected
        {
            return Err(ResourceError::UnexpectedModel);
        }
        response
            .available_hotkeys
            .into_iter()
            .map(|hotkey| {
                valid_text(&hotkey.hotkey_id)?;
                valid_text(&hotkey.name)?;
                valid_text(&hotkey.kind)?;
                Ok(VtsHotkeyInfo {
                    id: hotkey.hotkey_id,
                    name: hotkey.name,
                    kind: hotkey.kind,
                })
            })
            .collect()
    }

    pub async fn current_model_id(&mut self) -> Result<Option<String>, ResourceError> {
        let data = self
            .request("CurrentModelRequest", "CurrentModelResponse", json!({}))
            .await?;
        match data["modelLoaded"].as_bool() {
            Some(false) => Ok(None),
            Some(true) => {
                let model_id = data["modelID"]
                    .as_str()
                    .ok_or(ResourceError::InvalidResponse)?;
                valid_text(model_id)?;
                Ok(Some(model_id.to_owned()))
            }
            None => Err(ResourceError::InvalidResponse),
        }
    }

    pub async fn preview_hotkey(
        &mut self,
        expected_model_id: &str,
        hotkey_id: &str,
    ) -> Result<(), ResourceError> {
        valid_text(expected_model_id)?;
        valid_text(hotkey_id)?;
        if self.current_model_id().await?.as_deref() != Some(expected_model_id) {
            return Err(ResourceError::CurrentModelChanged);
        }
        let data = self
            .request(
                "HotkeyTriggerRequest",
                "HotkeyTriggerResponse",
                json!({"hotkeyID":hotkey_id}),
            )
            .await?;
        if data["hotkeyID"].as_str() != Some(hotkey_id) {
            return Err(ResourceError::UnexpectedHotkey);
        }
        Ok(())
    }

    async fn request(
        &mut self,
        kind: &str,
        response: &str,
        data: Value,
    ) -> Result<Value, ResourceError> {
        self.client
            .request(kind, response, data, self.deadline)
            .await
            .map_err(map_client_error)
    }
}

fn valid_text(value: &str) -> Result<(), ResourceError> {
    if value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
        return Err(ResourceError::InvalidResponse);
    }
    Ok(())
}

fn map_client_error(error: ClientError) -> ResourceError {
    match error {
        ClientError::Authorization => ResourceError::AuthorizationRequired,
        ClientError::Timeout => ResourceError::RequestTimeout,
        ClientError::Api(code) => ResourceError::Api(code),
        ClientError::Protocol(_) => ResourceError::InvalidResponse,
        ClientError::Storage(message) => ResourceError::InvalidConfiguration(message),
        ClientError::Transport | ClientError::Shutdown | ClientError::Reset => {
            ResourceError::ConnectionFailed
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AvailableModels {
    number_of_models: u64,
    available_models: Vec<ModelResponse>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelResponse {
    model_loaded: bool,
    model_name: String,
    #[serde(rename = "modelID")]
    model_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AvailableHotkeys {
    #[serde(rename = "modelID")]
    model_id: String,
    available_hotkeys: Vec<HotkeyResponse>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HotkeyResponse {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "hotkeyID")]
    hotkey_id: String,
}
