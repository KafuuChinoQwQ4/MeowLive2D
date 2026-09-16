//! 16 字节大端 WS 包解析、有界解压和开放平台通知映射。

use meowlive_domain::event::{EventKind, LiveEvent};
use serde::Deserialize;
use std::io::Read;

const HEADER_LEN: usize = 16;
const OP_NOTIFY: u32 = 5;

pub fn api_retry_delay(code: i64) -> Option<std::time::Duration> {
    (code == 7001).then(|| std::time::Duration::from_secs(10))
}

#[derive(Clone, Copy, Debug)]
pub struct DecodeLimits {
    pub max_frame_bytes: usize,
    pub max_decompressed_bytes: usize,
    pub max_packets: usize,
    pub max_compression_depth: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 256 * 1024,
            max_decompressed_bytes: 1024 * 1024,
            max_packets: 256,
            max_compression_depth: 2,
        }
    }
}

impl DecodeLimits {
    pub fn validate(self) -> Result<(), String> {
        if !(HEADER_LEN..=1024 * 1024).contains(&self.max_frame_bytes)
            || !(HEADER_LEN..=4 * 1024 * 1024).contains(&self.max_decompressed_bytes)
            || !(1..=1024).contains(&self.max_packets)
            || !(1..=4).contains(&self.max_compression_depth)
        {
            return Err("哔哩哔哩消息解析上限无效".into());
        }
        Ok(())
    }
}

pub fn packet(operation: u32, version: u16, body: &[u8]) -> Result<Vec<u8>, String> {
    let length = HEADER_LEN
        .checked_add(body.len())
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| "哔哩哔哩消息过大".to_string())?;
    let mut bytes = Vec::with_capacity(length as usize);
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(&(HEADER_LEN as u16).to_be_bytes());
    bytes.extend_from_slice(&version.to_be_bytes());
    bytes.extend_from_slice(&operation.to_be_bytes());
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.extend_from_slice(body);
    Ok(bytes)
}

pub fn decode_events(
    bytes: &[u8],
    room_id: &str,
    limits: DecodeLimits,
) -> Result<Vec<LiveEvent>, String> {
    Ok(decode_frame(bytes, room_id, None, limits)?.events)
}

pub(crate) struct DecodedFrame {
    pub events: Vec<LiveEvent>,
    pub interaction_ended: bool,
}

pub(crate) fn decode_frame(
    bytes: &[u8],
    room_id: &str,
    game_id: Option<&str>,
    limits: DecodeLimits,
) -> Result<DecodedFrame, String> {
    limits.validate()?;
    if bytes.len() > limits.max_frame_bytes {
        return Err("哔哩哔哩消息超过帧上限".into());
    }
    let mut decoded = DecodedFrame {
        events: Vec::new(),
        interaction_ended: false,
    };
    let mut budget = DecodeBudget {
        packets: 0,
        remaining: limits.max_decompressed_bytes,
    };
    decode_into(
        bytes,
        room_id,
        game_id,
        limits,
        0,
        &mut budget,
        &mut decoded,
    )?;
    Ok(decoded)
}

struct DecodeBudget {
    packets: usize,
    remaining: usize,
}

fn decode_into(
    bytes: &[u8],
    room_id: &str,
    game_id: Option<&str>,
    limits: DecodeLimits,
    depth: usize,
    budget: &mut DecodeBudget,
    decoded: &mut DecodedFrame,
) -> Result<(), String> {
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes.len() - offset < HEADER_LEN {
            return Err("哔哩哔哩消息包头不完整".into());
        }
        budget.packets += 1;
        if budget.packets > limits.max_packets {
            return Err("哔哩哔哩消息包数量超过上限".into());
        }
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let header = u16::from_be_bytes(bytes[offset + 4..offset + 6].try_into().unwrap()) as usize;
        let version = u16::from_be_bytes(bytes[offset + 6..offset + 8].try_into().unwrap());
        let operation = u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap());
        if header < HEADER_LEN || length < header || length > bytes.len() - offset {
            return Err("哔哩哔哩消息包长度无效".into());
        }
        let body = &bytes[offset + header..offset + length];
        match version {
            0 | 1 if operation == OP_NOTIFY => match map_notice(body, room_id, game_id) {
                NoticeOutcome::Event(event) => decoded.events.push(event),
                NoticeOutcome::InteractionEnded => decoded.interaction_ended = true,
                NoticeOutcome::Ignored => {}
            },
            0 | 1 => {}
            2 | 3 => {
                if depth >= limits.max_compression_depth {
                    return Err("哔哩哔哩压缩嵌套超过上限".into());
                }
                let expanded = decompress(body, version, budget.remaining)?;
                budget.remaining -= expanded.len();
                decode_into(
                    &expanded,
                    room_id,
                    game_id,
                    limits,
                    depth + 1,
                    budget,
                    decoded,
                )?;
            }
            _ => return Err("哔哩哔哩消息协议版本不受支持".into()),
        }
        offset += length;
    }
    Ok(())
}

fn decompress(input: &[u8], version: u16, maximum: usize) -> Result<Vec<u8>, String> {
    let reader: Box<dyn Read> = match version {
        2 => Box::new(flate2::read::ZlibDecoder::new(input)),
        3 => Box::new(brotli::Decompressor::new(input, 4096)),
        _ => unreachable!(),
    };
    let mut output = Vec::new();
    reader
        .take(maximum as u64 + 1)
        .read_to_end(&mut output)
        .map_err(|_| "哔哩哔哩压缩消息无效".to_string())?;
    if output.len() > maximum {
        return Err("哔哩哔哩解压消息超过上限".into());
    }
    Ok(output)
}

pub(crate) fn authority_result(bytes: &[u8], maximum: usize) -> Result<Option<bool>, String> {
    if bytes.len() > maximum || bytes.len() < HEADER_LEN {
        return Err("哔哩哔哩鉴权响应大小无效".into());
    }
    let mut offset = 0;
    let mut authority = None;
    while offset < bytes.len() {
        let remaining = &bytes[offset..];
        if remaining.len() < HEADER_LEN {
            return Err("哔哩哔哩鉴权响应包头不完整".into());
        }
        let length = u32::from_be_bytes(remaining[0..4].try_into().unwrap()) as usize;
        let header = u16::from_be_bytes(remaining[4..6].try_into().unwrap()) as usize;
        let operation = u32::from_be_bytes(remaining[8..12].try_into().unwrap());
        if header < HEADER_LEN || length < header || length > remaining.len() {
            return Err("哔哩哔哩鉴权响应包长度无效".into());
        }
        if operation == 8 {
            let body = &remaining[header..length];
            #[derive(Deserialize)]
            struct AuthorityResponse {
                code: Option<i64>,
            }
            let accepted = if body.is_empty() {
                true
            } else {
                let response: AuthorityResponse = serde_json::from_slice(body)
                    .map_err(|_| "哔哩哔哩鉴权响应格式无效".to_string())?;
                response.code.is_none_or(|code| code == 0)
            };
            if !accepted {
                return Ok(Some(false));
            }
            authority = Some(true);
        }
        offset += length;
    }
    Ok(authority)
}

#[derive(Deserialize)]
struct NoticeData {
    room_id: u64,
    msg_id: String,
    uname: String,
    timestamp: u64,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    gift_name: Option<String>,
    #[serde(default)]
    gift_num: Option<u64>,
}

enum NoticeOutcome {
    Event(LiveEvent),
    InteractionEnded,
    Ignored,
}

fn map_notice(body: &[u8], expected_room: &str, game_id: Option<&str>) -> NoticeOutcome {
    let Ok(envelope) = serde_json::from_slice::<serde_json::Value>(body) else {
        return NoticeOutcome::Ignored;
    };
    let Some(command) = envelope.get("cmd").and_then(|value| value.as_str()) else {
        return NoticeOutcome::Ignored;
    };
    if command == "LIVE_OPEN_PLATFORM_INTERACTION_END" {
        let ended_game = envelope
            .pointer("/data/game_id")
            .and_then(|value| value.as_str());
        return if game_id.is_some() && ended_game == game_id {
            NoticeOutcome::InteractionEnded
        } else {
            NoticeOutcome::Ignored
        };
    }
    let Ok(data) = serde_json::from_value::<NoticeData>(envelope["data"].clone()) else {
        return NoticeOutcome::Ignored;
    };
    if data.room_id.to_string() != expected_room || data.msg_id.trim().is_empty() {
        return NoticeOutcome::Ignored;
    }
    let id = format!("bilibili:{expected_room}:{}", data.msg_id);
    let kind = match command {
        "LIVE_OPEN_PLATFORM_DM" => match data.msg {
            Some(text) => EventKind::Chat { text },
            None => return NoticeOutcome::Ignored,
        },
        "LIVE_OPEN_PLATFORM_SEND_GIFT" => match (data.gift_name, data.gift_num) {
            (Some(name), Some(count)) => match u32::try_from(count) {
                Ok(count) => EventKind::Gift { name, count },
                Err(_) => return NoticeOutcome::Ignored,
            },
            _ => return NoticeOutcome::Ignored,
        },
        _ => return NoticeOutcome::Ignored,
    };
    let event = LiveEvent {
        id,
        source: "bilibili".into(),
        viewer: data.uname,
        occurred_at_ms: data.timestamp.saturating_mul(1000),
        kind,
    };
    if event.validate().is_ok() {
        NoticeOutcome::Event(event)
    } else {
        NoticeOutcome::Ignored
    }
}
