//! OBS WebSocket v5 的本地有界控制；音频采集和推流由 OBS 本身负责。
mod config;
mod settings;
pub use config::ObsConfig;
pub use settings::{save_settings, settings};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use meowlive_protocol::obs::{ObsOperation, ObsSnapshot};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{Error, Message, protocol::WebSocketConfig},
};

const MAX_FRAME: usize = 256 * 1024;
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

pub async fn execute(config: &ObsConfig, operation: ObsOperation) -> Result<ObsSnapshot, String> {
    let (config, password) = settings::resolve(config).await?;
    config.validate()?;
    if !config.enabled {
        return Ok(ObsSnapshot {
            connected: false,
            recording: false,
            current_scene: String::new(),
            scenes: Vec::new(),
        });
    }
    tokio::time::timeout(
        Duration::from_millis(config.timeout_ms),
        execute_inner(&config, password.as_deref(), operation),
    )
    .await
    .map_err(|_| "OBS 操作超时".to_owned())?
}

async fn execute_inner(
    config: &ObsConfig,
    password: Option<&str>,
    operation: ObsOperation,
) -> Result<ObsSnapshot, String> {
    let limits = WebSocketConfig::default()
        .max_message_size(Some(MAX_FRAME))
        .max_frame_size(Some(MAX_FRAME));
    let (mut ws, _) = connect_async_with_config(&config.websocket_url, Some(limits), false)
        .await
        .map_err(|_| "无法连接 OBS WebSocket".to_owned())?;
    let hello = recv(&mut ws).await?;
    if hello.get("op").and_then(Value::as_u64) != Some(0) {
        return Err("OBS Hello 消息无效".into());
    }
    let d = hello
        .get("d")
        .and_then(Value::as_object)
        .ok_or("OBS Hello 数据缺失")?;
    if d.get("rpcVersion")
        .and_then(Value::as_u64)
        .is_none_or(|version| version < 1)
    {
        return Err("OBS RPC 版本无效".into());
    }
    let mut identify = json!({"op":1,"d":{"rpcVersion":1}});
    if let Some(auth) = d.get("authentication") {
        let salt = auth
            .get("salt")
            .and_then(Value::as_str)
            .ok_or("OBS 鉴权 salt 无效")?;
        let challenge = auth
            .get("challenge")
            .and_then(Value::as_str)
            .ok_or("OBS 鉴权 challenge 无效")?;
        if salt.is_empty() || challenge.is_empty() {
            return Err("OBS 鉴权挑战无效".into());
        }
        let password =
            password.ok_or("OBS 要求鉴权，请在连接设置中保存密码（初始密码环境变量缺失）")?;
        identify["d"]["authentication"] = json!(authentication(password, salt, challenge));
    }
    send(&mut ws, identify).await?;
    let identified = recv(&mut ws).await?;
    if identified.get("op").and_then(Value::as_u64) != Some(2)
        || identified
            .pointer("/d/negotiatedRpcVersion")
            .and_then(Value::as_u64)
            != Some(1)
    {
        return Err("OBS 鉴权或 RPC 协商失败".into());
    }
    let snapshot = read_snapshot(&mut ws).await?;
    match operation {
        ObsOperation::Status => return Ok(snapshot),
        ObsOperation::SetScene { scene_name } => {
            validate_scene(&scene_name)?;
            if !snapshot.scenes.iter().any(|name| name == &scene_name) {
                return Err("OBS 场景不存在".into());
            }
            if snapshot.current_scene != scene_name {
                request(
                    &mut ws,
                    "SetCurrentProgramScene",
                    json!({"sceneName":scene_name}),
                )
                .await?;
            }
        }
        ObsOperation::StartRecording if !snapshot.recording => {
            request(&mut ws, "StartRecord", json!({})).await?;
        }
        ObsOperation::StopRecording if snapshot.recording => {
            request(&mut ws, "StopRecord", json!({})).await?;
        }
        ObsOperation::StartRecording | ObsOperation::StopRecording => return Ok(snapshot),
    }
    read_snapshot(&mut ws).await
}

async fn read_snapshot(
    ws: &mut (
             impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error>
             + StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
             + Unpin
         ),
) -> Result<ObsSnapshot, String> {
    let status = request(ws, "GetRecordStatus", json!({})).await?;
    let recording = status
        .get("outputActive")
        .and_then(Value::as_bool)
        .ok_or("OBS 录制状态无效")?;
    let scenes_value = request(ws, "GetSceneList", json!({})).await?;
    let scenes = scenes_value
        .get("scenes")
        .and_then(Value::as_array)
        .ok_or("OBS 场景列表无效")?;
    if scenes.len() > 256 {
        return Err("OBS 场景数量超限".into());
    }
    let mut names = Vec::with_capacity(scenes.len());
    for scene in scenes {
        let name = scene
            .get("sceneName")
            .and_then(Value::as_str)
            .ok_or("OBS 场景名称无效")?;
        validate_scene(name)?;
        names.push(name.to_owned());
    }
    let current = scenes_value
        .get("currentProgramSceneName")
        .and_then(Value::as_str)
        .ok_or("OBS 当前场景无效")?;
    validate_scene(current)?;
    Ok(ObsSnapshot {
        connected: true,
        recording,
        current_scene: current.to_owned(),
        scenes: names,
    })
}
fn validate_scene(s: &str) -> Result<(), String> {
    if s.is_empty() || s.chars().count() > 256 || s.chars().any(char::is_control) {
        Err("OBS 场景名称无效".into())
    } else {
        Ok(())
    }
}
fn authentication(password: &str, salt: &str, challenge: &str) -> String {
    let secret = STANDARD.encode(Sha256::digest(format!("{password}{salt}").as_bytes()));
    STANDARD.encode(Sha256::digest(format!("{secret}{challenge}").as_bytes()))
}
async fn send(
    ws: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    value: Value,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(&value).map_err(|_| "OBS 请求编码失败")?;
    if bytes.len() > MAX_FRAME {
        return Err("OBS 请求帧超限".into());
    }
    ws.send(Message::Text(
        String::from_utf8(bytes)
            .map_err(|_| "OBS 请求编码失败")?
            .into(),
    ))
    .await
    .map_err(|_| "OBS 请求发送失败".into())
}
async fn recv(
    ws: &mut (
             impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error>
             + StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
             + Unpin
         ),
) -> Result<Value, String> {
    loop {
        let message = ws
            .next()
            .await
            .ok_or("OBS 连接已关闭")?
            .map_err(|error| match error {
                Error::Capacity(_) => "OBS 响应帧超限",
                _ => "OBS 响应读取失败",
            })?;
        let bytes = match message {
            Message::Text(text) => text.as_bytes().to_vec(),
            Message::Ping(payload) => {
                ws.send(Message::Pong(payload))
                    .await
                    .map_err(|_| "OBS Pong 发送失败")?;
                continue;
            }
            Message::Pong(_) => continue,
            Message::Binary(_) => return Err("OBS 响应帧类型无效".into()),
            Message::Close(_) => return Err("OBS 连接已关闭".into()),
            _ => continue,
        };
        if bytes.len() > MAX_FRAME {
            return Err("OBS 响应帧超限".into());
        }
        return serde_json::from_slice(&bytes).map_err(|_| "OBS 响应 JSON 无效".into());
    }
}
async fn request(
    ws: &mut (
             impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error>
             + StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
             + Unpin
         ),
    kind: &str,
    data: Value,
) -> Result<Value, String> {
    let id = format!("meow-{}", NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed));
    send(
        ws,
        json!({"op":6,"d":{"requestType":kind,"requestId":id,"requestData":data}}),
    )
    .await?;
    loop {
        let response = recv(ws).await?;
        match response.get("op").and_then(Value::as_u64) {
            Some(5) => {
                let data = response
                    .get("d")
                    .and_then(Value::as_object)
                    .ok_or("OBS 事件无效")?;
                if data
                    .get("eventType")
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
                    || data.get("eventIntent").and_then(Value::as_u64).is_none()
                    || data.get("eventData").and_then(Value::as_object).is_none()
                {
                    return Err("OBS 事件无效".into());
                }
                continue;
            }
            Some(7) => {}
            _ => return Err("OBS 响应操作码无效".into()),
        }
        let d = response
            .get("d")
            .and_then(Value::as_object)
            .ok_or("OBS 响应数据缺失")?;
        if d.get("requestId").and_then(Value::as_str) != Some(&id)
            || d.get("requestType").and_then(Value::as_str) != Some(kind)
        {
            return Err("OBS 响应 ID 或类型不匹配".into());
        }
        let status = d
            .get("requestStatus")
            .and_then(Value::as_object)
            .ok_or("OBS 请求状态无效")?;
        let result = status
            .get("result")
            .and_then(Value::as_bool)
            .ok_or("OBS 请求状态无效")?;
        let code = status
            .get("code")
            .and_then(Value::as_u64)
            .ok_or("OBS 请求状态无效")?;
        if !result {
            return Err(format!("OBS 请求 {kind} 被拒绝（代码 {code}）"));
        }
        if code != 100 {
            return Err("OBS 请求状态无效".into());
        }
        return Ok(d.get("responseData").cloned().unwrap_or_else(|| json!({})));
    }
}

#[cfg(test)]
mod tests {
    use super::authentication;

    #[test]
    fn authentication_applies_the_documented_double_sha256_base64_algorithm() {
        assert_eq!(
            authentication(
                "supersecretpassword",
                "lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=",
                "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY="
            ),
            "1Ct943GAT+6YQUUX47Ia/ncufilbe6+oD6lY+5kaCu4="
        );
    }
}
