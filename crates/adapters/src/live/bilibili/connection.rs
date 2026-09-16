//! LiveSource 与 LiveConnection 的有界网络生命周期实现。

use super::{
    BilibiliConfig,
    config::loopback_host,
    http::{ApiClient, StartedProject, error},
    protocol::{self, DecodedFrame},
};
use futures_util::{SinkExt, StreamExt};
use meowlive_application::ports::live_source::{
    LiveConnection, LiveFuture, LiveSource, LiveSourceError,
};
use meowlive_domain::event::LiveEvent;
use std::{collections::VecDeque, time::Duration};
use tokio::{net::TcpStream, time::Instant};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, protocol::WebSocketConfig},
};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct BilibiliLiveSource {
    config: BilibiliConfig,
}

impl BilibiliLiveSource {
    pub fn new(config: BilibiliConfig) -> Result<Self, LiveSourceError> {
        config
            .validate()
            .map_err(|message| LiveSourceError::new(message, false))?;
        Ok(Self { config })
    }
}

impl LiveSource for BilibiliLiveSource {
    fn connect(&self) -> LiveFuture<'_, Box<dyn LiveConnection>> {
        Box::pin(async move {
            let api = ApiClient::new(self.config.clone())?;
            let project = api.start().await?;
            match connect_socket(&self.config, &project).await {
                Ok((socket, initial)) => Ok(Box::new(BilibiliConnection::new(
                    self.config.clone(),
                    api,
                    project,
                    socket,
                    initial,
                )) as Box<dyn LiveConnection>),
                Err(connect_error) => {
                    api.end(&project.game_id).await.map_err(|_| {
                        error("长连接失败且结束直播会话失败，请核对平台会话", false)
                    })?;
                    Err(connect_error)
                }
            }
        })
    }
}

struct BilibiliConnection {
    config: BilibiliConfig,
    api: ApiClient,
    socket: Socket,
    game_id: String,
    room_id: String,
    pending: VecDeque<LiveEvent>,
    interaction_ended: bool,
    last_received: Instant,
    app_heartbeat_due: Instant,
    app_heartbeat: Option<futures_util::future::BoxFuture<'static, Result<(), LiveSourceError>>>,
    websocket_heartbeat_due: Instant,
    closed: bool,
}

impl BilibiliConnection {
    fn new(
        config: BilibiliConfig,
        api: ApiClient,
        project: StartedProject,
        socket: Socket,
        initial: DecodedFrame,
    ) -> Self {
        let now = Instant::now();
        Self {
            app_heartbeat_due: now + config.app_heartbeat_interval,
            app_heartbeat: None,
            websocket_heartbeat_due: now + config.websocket_heartbeat_interval,
            last_received: now,
            config,
            api,
            socket,
            game_id: project.game_id,
            room_id: project.room_id,
            pending: initial.events.into(),
            interaction_ended: initial.interaction_ended,
            closed: false,
        }
    }

    async fn receive_next(&mut self) -> Result<Option<LiveEvent>, LiveSourceError> {
        if self.closed {
            return Ok(None);
        }
        if let Some(event) = self.pending.pop_front() {
            return Ok(Some(event));
        }
        if self.interaction_ended {
            return Err(error("哔哩哔哩互动场次已结束", false));
        }
        loop {
            let receive_deadline = self.last_received + self.config.receive_timeout;
            tokio::select! {
                message = self.socket.next() => {
                    match message {
                        Some(Ok(Message::Binary(bytes))) => {
                            self.last_received = Instant::now();
                            let decoded = protocol::decode_frame(
                                &bytes,
                                &self.room_id,
                                Some(&self.game_id),
                                self.config.decode_limits,
                            )
                                .map_err(|message| error(message, false))?;
                            self.pending.extend(decoded.events);
                            self.interaction_ended |= decoded.interaction_ended;
                            if let Some(event) = self.pending.pop_front() {
                                return Ok(Some(event));
                            }
                            if self.interaction_ended {
                                return Err(error("哔哩哔哩互动场次已结束", false));
                            }
                        }
                        Some(Ok(Message::Ping(bytes))) => {
                            self.last_received = Instant::now();
                            send(&mut self.socket, Message::Pong(bytes), self.config.request_timeout).await?;
                        }
                        Some(Ok(Message::Pong(_))) => self.last_received = Instant::now(),
                        Some(Ok(Message::Close(_))) | None => return Ok(None),
                        Some(Ok(Message::Text(text))) => {
                            if text.len() > self.config.decode_limits.max_frame_bytes {
                                return Err(error("哔哩哔哩文本消息超过上限", false));
                            }
                            self.last_received = Instant::now();
                        }
                        Some(Ok(Message::Frame(_))) => {}
                        Some(Err(_)) => return Err(error("哔哩哔哩长连接读取失败", true)),
                    }
                }
                _ = tokio::time::sleep_until(self.app_heartbeat_due), if self.app_heartbeat.is_none() => {
                    let api = self.api.clone();
                    let game = self.game_id.clone();
                    self.app_heartbeat = Some(Box::pin(async move { api.heartbeat(&game).await }));
                }
                result = async {
                    match self.app_heartbeat.as_mut() {
                        Some(heartbeat) => heartbeat.await,
                        None => std::future::pending().await,
                    }
                } => {
                    self.app_heartbeat = None;
                    result?;
                    self.app_heartbeat_due = Instant::now() + self.config.app_heartbeat_interval;
                }
                _ = tokio::time::sleep_until(self.websocket_heartbeat_due) => {
                    let heartbeat = protocol::packet(2, 1, &[])
                        .map_err(|message| error(message, false))?;
                    send(&mut self.socket, Message::Binary(heartbeat.into()), self.config.request_timeout).await?;
                    self.websocket_heartbeat_due = Instant::now() + self.config.websocket_heartbeat_interval;
                }
                _ = tokio::time::sleep_until(receive_deadline) => {
                    return Err(error("哔哩哔哩长连接接收超时", true));
                }
            }
        }
    }

    async fn close_inner(&mut self) -> Result<(), LiveSourceError> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        // Cancel an outstanding HTTP heartbeat before ending its project.
        self.app_heartbeat = None;
        let _ = tokio::time::timeout(self.config.request_timeout, self.socket.close(None)).await;
        self.api.end(&self.game_id).await
    }
}

impl LiveConnection for BilibiliConnection {
    fn room_id(&self) -> &str {
        &self.room_id
    }

    fn next(&mut self) -> LiveFuture<'_, Option<LiveEvent>> {
        Box::pin(self.receive_next())
    }

    fn close(&mut self) -> LiveFuture<'_, ()> {
        Box::pin(self.close_inner())
    }
}

async fn connect_socket(
    config: &BilibiliConfig,
    project: &StartedProject,
) -> Result<(Socket, DecodedFrame), LiveSourceError> {
    let ws_config = WebSocketConfig::default()
        .max_message_size(Some(config.decode_limits.max_frame_bytes))
        .max_frame_size(Some(config.decode_limits.max_frame_bytes));
    let urls = websocket_urls(&project.websocket_urls)?;
    let deadline = Instant::now() + config.connect_timeout;
    let mut last_error = error("哔哩哔哩长连接建立失败", true);
    for url in urls {
        let connect =
            tokio_tungstenite::connect_async_with_config(url.as_str(), Some(ws_config), false);
        let (mut socket, _) = match tokio::time::timeout_at(deadline, connect).await {
            Ok(Ok(connected)) => connected,
            Ok(Err(_)) => continue,
            Err(_) => return Err(error("哔哩哔哩长连接建立超时", true)),
        };
        let authority = protocol::packet(7, 1, project.auth_body.as_bytes())
            .map_err(|message| error(message, false))?;
        if !matches!(
            tokio::time::timeout_at(deadline, socket.send(Message::Binary(authority.into()))).await,
            Ok(Ok(()))
        ) {
            continue;
        }
        match await_authority(&mut socket, deadline, config, project).await {
            Ok(initial) => return Ok((socket, initial)),
            Err(cause) if !cause.retryable => return Err(cause),
            Err(cause) => last_error = cause,
        }
    }
    Err(last_error)
}

async fn await_authority(
    socket: &mut Socket,
    deadline: Instant,
    config: &BilibiliConfig,
    project: &StartedProject,
) -> Result<DecodedFrame, LiveSourceError> {
    loop {
        let message = tokio::time::timeout_at(deadline, socket.next())
            .await
            .map_err(|_| error("哔哩哔哩长连接鉴权超时", true))?;
        match message {
            Some(Ok(Message::Binary(bytes))) => {
                match protocol::authority_result(&bytes, config.decode_limits.max_frame_bytes)
                    .map_err(|message| error(message, false))?
                {
                    Some(true) => {
                        return protocol::decode_frame(
                            &bytes,
                            &project.room_id,
                            Some(&project.game_id),
                            config.decode_limits,
                        )
                        .map_err(|message| error(message, false));
                    }
                    Some(false) => return Err(error("哔哩哔哩长连接鉴权被拒绝", false)),
                    None => continue,
                }
            }
            Some(Ok(Message::Ping(bytes))) => {
                tokio::time::timeout_at(deadline, socket.send(Message::Pong(bytes)))
                    .await
                    .map_err(|_| error("哔哩哔哩长连接鉴权写入超时", true))?
                    .map_err(|_| error("哔哩哔哩长连接鉴权写入失败", true))?;
            }
            Some(Ok(Message::Close(_))) | None => {
                return Err(error("哔哩哔哩长连接在鉴权前关闭", true));
            }
            Some(Ok(_)) => {}
            Some(Err(_)) => return Err(error("哔哩哔哩长连接鉴权读取失败", true)),
        }
    }
}

async fn send(
    socket: &mut Socket,
    message: Message,
    timeout: Duration,
) -> Result<(), LiveSourceError> {
    tokio::time::timeout(timeout, socket.send(message))
        .await
        .map_err(|_| error("哔哩哔哩长连接写入超时", true))?
        .map_err(|_| error("哔哩哔哩长连接写入失败", true))
}

fn websocket_urls(urls: &[String]) -> Result<Vec<reqwest::Url>, LiveSourceError> {
    let parsed: Vec<_> = urls
        .iter()
        .take(8)
        .filter_map(|raw| reqwest::Url::parse(raw).ok())
        .filter(|url| url.scheme() == "wss" || (url.scheme() == "ws" && loopback_host(url)))
        .collect();
    if parsed.is_empty() {
        Err(error("哔哩哔哩未返回安全的长连接地址", false))
    } else {
        Ok(parsed)
    }
}
