//! VTS 私有协议封装：响应大小、关联 ID 和总请求期限均在此边界校验。
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{Message, protocol::WebSocketConfig},
};

const MAX_RESPONSE: usize = 64 * 1024;
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

pub(super) enum Error {
    Transport,
    Storage(String),
    Timeout,
    Protocol(&'static str),
    Api(i64),
    Authorization,
    Shutdown,
    Reset,
}
impl Error {
    pub(super) fn message(&self) -> String {
        match self {
            Self::Protocol(message) => (*message).into(),
            Self::Storage(message) => message.clone(),
            Self::Api(code) => format!("VTS API 请求被拒绝（错误码 {code}）"),
            _ => "VTS 连接失败".into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    api_name: String,
    api_version: String,
    #[serde(rename = "requestID")]
    request_id: String,
    message_type: String,
    data: Value,
}

pub(super) struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}
impl Client {
    pub(super) async fn connect(url: &str, deadline: Duration) -> Result<Self, Error> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_RESPONSE))
            .max_frame_size(Some(MAX_RESPONSE));
        let (socket, _) =
            tokio::time::timeout(deadline, connect_async_with_config(url, Some(config), true))
                .await
                .map_err(|_| Error::Timeout)?
                .map_err(|_| Error::Transport)?;
        Ok(Self { socket })
    }

    pub(super) async fn request(
        &mut self,
        kind: &str,
        response_kind: &str,
        data: Value,
        deadline: Duration,
    ) -> Result<Value, Error> {
        // One deadline covers socket writes, ping handling and unrelated response IDs.
        tokio::time::timeout(deadline, async {
            let id = format!(
                "MeowLive2D-{}",
                NEXT_REQUEST.fetch_add(1, Ordering::Relaxed)
            );
            let request = json!({"apiName":"VTubeStudioPublicAPI","apiVersion":"1.0",
                "requestID":id,"messageType":kind,"data":data});
            self.socket
                .send(Message::Text(request.to_string().into()))
                .await
                .map_err(|_| Error::Transport)?;
            loop {
                let message = self.socket.next().await.ok_or(Error::Transport)?.map_err(
                    |error| match error {
                        tokio_tungstenite::tungstenite::Error::Capacity(_) => {
                            Error::Protocol("VTS 响应超过大小限制")
                        }
                        _ => Error::Transport,
                    },
                )?;
                match message {
                    Message::Text(text) => {
                        let response: Response = serde_json::from_str(&text)
                            .map_err(|_| Error::Protocol("VTS 响应格式无效"))?;
                        if response.api_name != "VTubeStudioPublicAPI"
                            || response.api_version != "1.0"
                        {
                            return Err(Error::Protocol("VTS 响应 API 标识或版本不匹配"));
                        }
                        if response.request_id != id {
                            continue;
                        }
                        if response.message_type == "APIError" {
                            let code = response.data["errorID"]
                                .as_i64()
                                .ok_or(Error::Protocol("VTS 错误响应格式无效"))?;
                            return Err(if code == 50 {
                                Error::Authorization
                            } else {
                                Error::Api(code)
                            });
                        }
                        if response.message_type != response_kind {
                            return Err(Error::Protocol("VTS 响应消息类型不匹配"));
                        }
                        if !response.data.is_object() {
                            return Err(Error::Protocol("VTS 响应数据格式无效"));
                        }
                        return Ok(response.data);
                    }
                    Message::Ping(_) => self.socket.flush().await.map_err(|_| Error::Transport)?,
                    Message::Pong(_) => {}
                    Message::Close(_) => return Err(Error::Transport),
                    _ => return Err(Error::Protocol("VTS 响应不是 JSON 文本")),
                }
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    pub(super) async fn inject(
        &mut self,
        parameter: &str,
        level: f64,
        deadline: Duration,
    ) -> Result<(), Error> {
        self.request(
            "InjectParameterDataRequest",
            "InjectParameterDataResponse",
            json!({"mode":"set","parameterValues":[{"id":parameter,"value":level}]}),
            deadline,
        )
        .await?;
        Ok(())
    }
}
