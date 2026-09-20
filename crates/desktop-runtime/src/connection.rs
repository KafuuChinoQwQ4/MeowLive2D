//! 主动连接主服务、接收控制与音频、回传结果和核对重连状态；不调用 LLM 或 TTS。

use crate::{audio::AudioBackend, config::ClientConfig, playback::Playback};
use futures_util::{SinkExt, StreamExt};
use meowlive_protocol::{
    PROTOCOL_VERSION,
    audio::{AudioChunk, MAX_AUDIO_FRAME_BYTES},
    control::{ClientMessage, ServerCommand},
    execution::ExecutionReceipt,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    time::{Duration, Instant, interval, sleep_until, timeout},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{Message, client::IntoClientRequest, protocol::WebSocketConfig},
};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

// Server heartbeats arrive every 10 seconds. Allow three missed intervals
// plus scheduling slack, independently for each paired channel.
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(35);

#[cfg(test)]
#[path = "../tests/connection/liveness.rs"]
mod liveness_tests;

#[cfg(test)]
#[path = "../tests/connection/flush.rs"]
mod flush_tests;

fn socket_url(origin: &str, path: &str) -> Result<url::Url, String> {
    let mut url = url::Url::parse(origin).map_err(|error| error.to_string())?;
    url.set_scheme("ws").map_err(|()| "invalid server URL")?;
    url.set_path(path);
    Ok(url)
}

pub fn audio_url(origin: &str, session_id: &str, bridge_id: &str) -> Result<String, String> {
    let mut url = socket_url(origin, "/ws/audio")?;
    url.query_pairs_mut()
        .append_pair("session_id", session_id)
        .append_pair("bridge_id", bridge_id);
    Ok(url.into())
}

async fn connect(
    url: &str,
    maximum: usize,
    duration: Duration,
    token: Option<&str>,
) -> Result<Socket, String> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(maximum))
        .max_frame_size(Some(maximum));
    let mut request = url
        .into_client_request()
        .map_err(|_| "invalid WebSocket request")?;
    if let Some(token) = token {
        let mut header = format!("Bearer {token}")
            .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
            .map_err(|_| "invalid device credential")?;
        header.set_sensitive(true);
        request.headers_mut().insert("authorization", header);
    }
    let (socket, _) = timeout(
        duration,
        connect_async_with_config(request, Some(config), false),
    )
    .await
    .map_err(|_| "WebSocket connect timed out")?
    .map_err(|error| error.to_string())?;
    Ok(socket)
}

fn read_device_token(path: Option<&std::path::Path>) -> Result<Option<String>, String> {
    use std::io::Read;
    let Some(path) = path else {
        return Ok(None);
    };
    let mut content = String::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(1025).read_to_string(&mut content))
        .map_err(|_| "cannot read private device credential")?;
    let token = content.trim();
    if !(32..=512).contains(&token.len()) || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("invalid private device credential".into());
    }
    Ok(Some(token.to_owned()))
}

async fn send<S: AsyncRead + AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
    message: ClientMessage,
    duration: Duration,
) -> Result<(), String> {
    let text = serde_json::to_string(&message).map_err(|error| error.to_string())?;
    timeout(duration, socket.send(Message::text(text)))
        .await
        .map_err(|_| "control write timed out")?
        .map_err(|error| error.to_string())
}

async fn receipts<S: AsyncRead + AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
    values: Vec<ExecutionReceipt>,
    duration: Duration,
) -> Result<(), String> {
    for receipt in values {
        send(socket, ClientMessage::Receipt { receipt }, duration).await?;
    }
    Ok(())
}

/// One connection generation. On any exit or cancellation, Playback::drop stops
/// the device and both owned sockets are dropped. Reconnect never resumes audio.
pub async fn run_once<B: AudioBackend>(config: &ClientConfig, backend: B) -> Result<(), String> {
    run_once_with_mouth(config, backend, None).await
}

pub async fn run_once_with_mouth<B: AudioBackend>(
    config: &ClientConfig,
    backend: B,
    mouth: Option<crate::presentation::MouthControl>,
) -> Result<(), String> {
    let duration = Duration::from_millis(config.handshake_timeout_ms);
    let device_token = read_device_token(config.device_token_file.as_deref())?;
    let mut control = connect(
        socket_url(&config.server_url, "/ws/control")?.as_str(),
        65_536,
        duration,
        device_token.as_deref(),
    )
    .await?;
    send(
        &mut control,
        ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
        },
        duration,
    )
    .await?;
    let message = timeout(duration, control.next())
        .await
        .map_err(|_| "server hello timed out")?
        .ok_or("control disconnected before hello")?
        .map_err(|error| error.to_string())?;
    let hello: ServerCommand =
        serde_json::from_str(message.to_text().map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let ServerCommand::Hello {
        protocol_version,
        session_id,
        bridge_id,
        generation,
    } = hello
    else {
        return Err("first control message must be hello".into());
    };
    if protocol_version != PROTOCOL_VERSION || session_id.is_empty() || bridge_id.is_empty() {
        return Err("server protocol version or pairing identity is invalid".into());
    }
    let audio = connect(
        &audio_url(&config.server_url, &session_id, &bridge_id)?,
        MAX_AUDIO_FRAME_BYTES,
        duration,
        device_token.as_deref(),
    )
    .await?;
    run_paired_resources(
        control,
        audio,
        backend,
        generation,
        config.max_buffer_samples,
        (duration, HEARTBEAT_TIMEOUT),
        Some(crate::resource_control::ResourceContext {
            config: config.clone(),
            mouth,
        }),
    )
    .await
}

async fn flush<S: AsyncRead + AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
    duration: Duration,
    channel: &str,
) -> Result<(), String> {
    timeout(duration.min(Duration::from_secs(1)), socket.flush())
        .await
        .map_err(|_| format!("{channel} heartbeat write timed out"))?
        .map_err(|error| error.to_string())
}

async fn run_paired_resources<B, C, A>(
    mut control: WebSocketStream<C>,
    mut audio: WebSocketStream<A>,
    backend: B,
    generation: u32,
    maximum: usize,
    timing: (Duration, Duration),
    resources: Option<crate::resource_control::ResourceContext>,
) -> Result<(), String>
where
    B: AudioBackend,
    C: AsyncRead + AsyncWrite + Unpin,
    A: AsyncRead + AsyncWrite + Unpin,
{
    let (duration, heartbeat_timeout) = timing;
    let mut operations = tokio::task::JoinSet::new();
    let mut player = Playback::new(backend, generation, maximum);
    let mut clock = interval(Duration::from_millis(5));
    let mut control_deadline = Instant::now() + heartbeat_timeout;
    let mut audio_deadline = Instant::now() + heartbeat_timeout;
    loop {
        tokio::select! {
            biased;
            _ = sleep_until(control_deadline) => return Err("control heartbeat timed out".into()),
            _ = sleep_until(audio_deadline) => return Err("audio heartbeat timed out".into()),
            incoming = control.next() => {
                control_deadline = Instant::now() + heartbeat_timeout;
                match incoming.ok_or("control disconnected")?.map_err(|error| error.to_string())? {
                    Message::Text(text) => {
                        let command = serde_json::from_str(&text).map_err(|error| format!("invalid control command: {error}"))?;
                        match command {
                            ServerCommand::Resource {request_id, operation} => {
                                if request_id.is_empty() || request_id.len() > 64 { return Err("invalid resource correlation".into()); }
                                if !operations.is_empty() {
                                    send(&mut control, ClientMessage::ResourceResult {request_id, result: meowlive_protocol::resources::DesktopResourceResult::Error{code:"resource_busy".into(),message:"桌面资源操作正在进行".into()}}, duration).await?;
                                } else {
                                    let resources=resources.clone();
                                    operations.spawn(async move {
                                        let result=match resources {
                                            Some(resources)=>tokio::time::timeout(Duration::from_secs(85),resources.execute(operation)).await.unwrap_or_else(|_|meowlive_protocol::resources::DesktopResourceResult::Error{code:"resource_timeout".into(),message:"桌面资源操作超时，请检查模型状态".into()}),
                                            None=>meowlive_protocol::resources::DesktopResourceResult::Error{code:"resource_unavailable".into(),message:"桌面资源执行器未配置".into()},
                                        };
                                        ClientMessage::ResourceResult { request_id, result }
                                    });
                                }
                            }
                            command => {
                                if matches!(command, ServerCommand::Stop{..}) { operations.abort_all(); }
                                receipts(&mut control, player.command(command)?, duration).await?;
                            }
                        }
                    }
                    Message::Close(_) => return Err("control disconnected".into()),
                    Message::Ping(_) | Message::Pong(_) => { flush(&mut control, duration, "control").await?; }
                    _ => return Err("control connection requires JSON text".into()),
                }
            }
            result = operations.join_next(), if !operations.is_empty() => {
                if let Some(Ok(message))=result {send(&mut control,message,duration).await?;}
            }
            _ = clock.tick() => { receipts(&mut control, player.poll(), duration).await?; }
            incoming = audio.next() => {
                audio_deadline = Instant::now() + heartbeat_timeout;
                match incoming.ok_or("audio disconnected")?.map_err(|error| error.to_string())? {
                    Message::Binary(bytes) => { receipts(&mut control, player.chunk(AudioChunk::decode(&bytes)?)?, duration).await?; }
                    Message::Close(_) => return Err("audio disconnected".into()),
                    Message::Ping(_) | Message::Pong(_) => { flush(&mut audio, duration, "audio").await?; }
                    _ => return Err("audio connection requires binary PCM".into()),
                }
            }
        }
    }
}

#[cfg(test)]
async fn run_paired<B, C, A>(
    control: WebSocketStream<C>,
    audio: WebSocketStream<A>,
    backend: B,
    generation: u32,
    maximum: usize,
    duration: Duration,
    heartbeat_timeout: Duration,
) -> Result<(), String>
where
    B: AudioBackend,
    C: AsyncRead + AsyncWrite + Unpin,
    A: AsyncRead + AsyncWrite + Unpin,
{
    run_paired_resources(
        control,
        audio,
        backend,
        generation,
        maximum,
        (duration, heartbeat_timeout),
        None,
    )
    .await
}
