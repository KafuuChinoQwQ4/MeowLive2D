mod bilibili_support;

use bilibili_support::{brotli_packet, notify_packet, packet, zlib_packet};
use meowlive_adapters::live::bilibili::protocol::{
    DecodeLimits, api_retry_delay, decode_events, decode_events_with_app_id,
};
use meowlive_domain::event::{EventKind, GiftMetadata, ViewerIdentity, ViewerIdentityKind};

fn super_chat_notice() -> serde_json::Value {
    serde_json::json!({
        "cmd": "LIVE_OPEN_PLATFORM_SUPER_CHAT",
        "data": {
            "room_id": 99,
            "message_id": 12345,
            "uname": "Alice",
            "uid": 42,
            "open_id": "open-user",
            "timestamp": 1700000000,
            "message": "请介绍一下今天的主题",
            "rmb": 30,
            "start_time": 1700000000,
            "end_time": 1700000060,
            "price": 30000,
            "paid": true,
            "fans_medal_level": 12,
            "guard_level": 3
        }
    })
}

fn notice_bytes(notice: &serde_json::Value) -> Vec<u8> {
    notify_packet(&serde_json::to_vec(notice).unwrap())
}

#[test]
fn maps_official_super_chat_without_msg_id_and_preserves_stable_identity() {
    let events = decode_events_with_app_id(
        &notice_bytes(&super_chat_notice()),
        "99",
        7,
        DecodeLimits::default(),
    )
    .unwrap();

    assert_eq!(events.len(), 1, "a valid official SC must be accepted");
    assert_eq!(events[0].id, "bilibili:99:sc:12345");
    assert_eq!(events[0].viewer, "Alice");
    assert_eq!(events[0].occurred_at_ms, 1_700_000_000_000);
    assert_eq!(events[0].gift_metadata, None);
    assert_eq!(
        events[0].kind,
        EventKind::SuperChat {
            text: "请介绍一下今天的主题".into(),
            amount_cny: 30,
            start_at_ms: 1_700_000_000_000,
            end_at_ms: 1_700_000_060_000,
        }
    );
    assert_eq!(
        events[0].viewer_identity,
        Some(ViewerIdentity {
            namespace: "bilibili:global".into(),
            kind: ViewerIdentityKind::Uid,
            external_id: "42".into(),
        })
    );
}

#[test]
fn maps_official_room_entry_without_chat_text() {
    let bytes = notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_LIVE_ROOM_ENTER","data":{"room_id":99,"msg_id":"enter-1","uname":"Bob","timestamp":1700000002,"uid":0,"open_id":"open-bob"}}"#);
    let events = decode_events_with_app_id(&bytes, "99", 7, DecodeLimits::default()).unwrap();

    assert_eq!(events.len(), 1, "a valid room entry must be accepted");
    assert_eq!(events[0].id, "bilibili:99:enter-1");
    assert_eq!(events[0].viewer, "Bob");
    assert_eq!(events[0].occurred_at_ms, 1_700_000_002_000);
    assert_eq!(events[0].gift_metadata, None);
    assert_eq!(events[0].kind, EventKind::RoomEnter);
    assert_eq!(
        events[0].viewer_identity,
        Some(ViewerIdentity {
            namespace: "bilibili:app:7".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "open-bob".into(),
        })
    );
}

#[test]
fn super_chat_identity_is_stable_across_retransmitted_notification_ids() {
    let mut first = super_chat_notice();
    first["data"]["uid"] = serde_json::json!(0);
    let mut second = first.clone();
    second["data"]["message_id"] = serde_json::json!("12345");
    second["data"]["msg_id"] = serde_json::json!("retransmitted-notice");
    let bytes = [
        zlib_packet(&notice_bytes(&first)),
        brotli_packet(&notice_bytes(&second)),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":99,"msg_id":"12345","uname":"Alice","timestamp":1700000000,"msg":"ordinary message"}}"#),
    ].concat();
    let events = decode_events_with_app_id(&bytes, "99", 7, DecodeLimits::default()).unwrap();

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].id, events[1].id);
    assert_ne!(events[0].id, events[2].id);
    assert_eq!(
        events[0].viewer_identity,
        Some(ViewerIdentity {
            namespace: "bilibili:app:7".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "open-user".into(),
        })
    );
}

#[test]
fn rejects_invalid_super_chat_money_text_ids_and_timestamps() {
    let invalid_fields = [
        ("rmb", serde_json::json!(0)),
        ("rmb", serde_json::json!(1_000_001)),
        ("rmb", serde_json::json!(-1)),
        ("rmb", serde_json::json!(30.5)),
        ("rmb", serde_json::json!(u64::MAX)),
        ("message", serde_json::json!(" ")),
        ("message", serde_json::json!("猫".repeat(501))),
        ("message", serde_json::json!("bad\u{0000}message")),
        ("message_id", serde_json::Value::Null),
        ("message_id", serde_json::json!(0)),
        ("message_id", serde_json::json!(" ")),
        ("message_id", serde_json::json!("x".repeat(128))),
        ("room_id", serde_json::json!(100)),
        ("timestamp", serde_json::json!(u64::MAX)),
        ("timestamp", serde_json::json!(9_007_199_254_741_u64)),
        ("start_time", serde_json::Value::Null),
        ("start_time", serde_json::json!(-1)),
        ("start_time", serde_json::json!(1_700_000_060)),
        ("end_time", serde_json::json!(1_699_999_999)),
        ("end_time", serde_json::json!(9_007_199_254_741_u64)),
        ("end_time", serde_json::json!(u64::MAX)),
    ];
    for (field, value) in invalid_fields {
        let mut notice = super_chat_notice();
        notice["data"][field] = value;
        let events = decode_events(&notice_bytes(&notice), "99", DecodeLimits::default()).unwrap();
        assert!(
            events.is_empty(),
            "invalid SC field {field} must be rejected"
        );
    }
}

#[test]
fn accepts_super_chat_unicode_and_money_upper_bound_without_truncation() {
    let mut notice = super_chat_notice();
    notice["data"]["message"] = serde_json::json!("猫".repeat(500));
    notice["data"]["rmb"] = serde_json::json!(1_000_000);
    let events = decode_events(&notice_bytes(&notice), "99", DecodeLimits::default()).unwrap();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(&events[0].kind, EventKind::SuperChat { text, amount_cny: 1_000_000, .. } if text.chars().count() == 500)
    );
}

#[test]
fn rejects_wrong_room_or_missing_entry_id_and_ignores_popularity() {
    let bytes = [
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_LIVE_ROOM_ENTER","data":{"room_id":100,"msg_id":"enter-1","uname":"Bob","timestamp":1700000002}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_LIVE_ROOM_ENTER","data":{"room_id":99,"uname":"Bob","timestamp":1700000002}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_LIVE_ROOM_ENTER","data":{"room_id":99,"msg_id":" ","uname":"Bob","timestamp":1700000002}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_LIVE_ROOM_ENTER","data":{"room_id":99,"msg_id":"enter-2","uname":"Bob","timestamp":9007199254741}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_HEAT","data":{"room_id":99,"msg_id":"heat-1","uname":"Bob","timestamp":1700000002,"popularity":100}}"#),
        packet(3, 1, &100_u32.to_be_bytes()),
    ].concat();
    assert!(
        decode_events(&bytes, "99", DecodeLimits::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn maps_official_chat_and_gift_fields_with_stable_scoped_ids() {
    let bytes = [
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":99,"msg_id":"dm-1","uname":"Alice","timestamp":1700000000,"msg":"hello"}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_SEND_GIFT","data":{"room_id":99,"msg_id":"gift-1","uname":"Bob","timestamp":1700000001,"gift_name":"Cat","gift_num":3}}"#),
    ].concat();
    let events = decode_events(&bytes, "99", DecodeLimits::default()).unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, "bilibili:99:dm-1");
    assert_eq!(events[0].source, "bilibili");
    assert_eq!(events[0].viewer, "Alice");
    assert_eq!(events[0].occurred_at_ms, 1_700_000_000_000);
    assert_eq!(
        events[0].kind,
        EventKind::Chat {
            text: "hello".into()
        }
    );
    assert_eq!(events[1].id, "bilibili:99:gift-1");
    assert_eq!(events[1].gift_metadata, None);
    assert_eq!(
        events[1].kind,
        EventKind::Gift {
            name: "Cat".into(),
            count: 3
        }
    );
}

#[test]
fn maps_stable_bilibili_identity_and_raw_gift_metadata() {
    let bytes = [
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":99,"msg_id":"dm-open","uname":"Renamed","timestamp":1700000000,"msg":"hello","open_id":"open-user"}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_SEND_GIFT","data":{"room_id":99,"msg_id":"gift-uid","uname":"Alice","timestamp":1700000001,"gift_name":"Cat","gift_num":3,"uid":42,"open_id":"different-open-id","price":1000,"paid":true,"fans_medal_level":12,"guard_level":3}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":99,"msg_id":"dm-zero-uid","uname":"Bob","timestamp":1700000002,"msg":"hello","uid":0,"open_id":"fallback-open-id"}}"#),
    ]
    .concat();
    let events = decode_events_with_app_id(&bytes, "99", 42, DecodeLimits::default()).unwrap();

    assert_eq!(
        events[0].viewer_identity,
        Some(ViewerIdentity {
            namespace: "bilibili:app:42".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "open-user".into(),
        })
    );
    assert_eq!(
        events[1].viewer_identity,
        Some(ViewerIdentity {
            namespace: "bilibili:global".into(),
            kind: ViewerIdentityKind::Uid,
            external_id: "42".into(),
        })
    );
    assert_eq!(
        events[1].gift_metadata,
        Some(GiftMetadata {
            price: Some(1_000),
            paid: Some(true),
            medal_level: Some(12),
            guard_level: Some(3),
        })
    );
    assert_eq!(
        events[2].viewer_identity,
        Some(ViewerIdentity {
            namespace: "bilibili:app:42".into(),
            kind: ViewerIdentityKind::OpenId,
            external_id: "fallback-open-id".into(),
        })
    );
}

#[test]
fn default_decode_does_not_invent_an_open_id_namespace() {
    let bytes = notify_packet(
        br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":99,"msg_id":"dm-open","uname":"Alice","timestamp":1700000000,"msg":"hello","open_id":"open-user"}}"#,
    );
    let events = decode_events(&bytes, "99", DecodeLimits::default()).unwrap();

    assert_eq!(events[0].viewer_identity, None);
}

#[test]
fn decodes_bounded_zlib_and_brotli_nested_packets() {
    let chat = notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":7,"msg_id":"one","uname":"A","timestamp":1,"msg":"z"}}"#);
    let gift = notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_SEND_GIFT","data":{"room_id":7,"msg_id":"two","uname":"B","timestamp":2,"gift_name":"G","gift_num":1}}"#);

    let bytes = [zlib_packet(&chat), brotli_packet(&gift)].concat();
    let events = decode_events(&bytes, "7", DecodeLimits::default()).unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>(),
        ["bilibili:7:one", "bilibili:7:two"]
    );
}

#[test]
fn rejects_malformed_or_oversized_frames_without_partial_output() {
    let malformed = packet(5, 0, b"{}");
    assert!(decode_events(&malformed[..10], "1", DecodeLimits::default()).is_err());

    let limits = DecodeLimits {
        max_frame_bytes: 17,
        ..DecodeLimits::default()
    };
    assert!(decode_events(&malformed, "1", limits).is_err());
}

#[test]
fn rejects_decompression_and_packet_count_expansion() {
    let inner = notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":1,"msg_id":"x","uname":"A","timestamp":1,"msg":"hello"}}"#);
    let compressed = zlib_packet(&inner);
    let small_output = DecodeLimits {
        max_decompressed_bytes: 32,
        ..DecodeLimits::default()
    };
    assert!(decode_events(&compressed, "1", small_output).is_err());

    let two_packets = [inner.clone(), inner].concat();
    let one_packet = DecodeLimits {
        max_packets: 1,
        ..DecodeLimits::default()
    };
    assert!(decode_events(&two_packets, "1", one_packet).is_err());
}

#[test]
fn ignores_unknown_or_invalid_notices() {
    let bytes = [
        notify_packet(br#"{"cmd":"OTHER","data":{}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_DM","data":{"room_id":2,"msg_id":"x","uname":"A","timestamp":1,"msg":"wrong room"}}"#),
        notify_packet(br#"{"cmd":"LIVE_OPEN_PLATFORM_SEND_GIFT","data":{"room_id":1,"msg_id":"x","uname":"A","timestamp":1,"gift_name":"G","gift_num":0}}"#),
    ].concat();
    assert!(
        decode_events(&bytes, "1", DecodeLimits::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn rate_limit_code_requires_the_official_minimum_retry_delay() {
    assert_eq!(
        api_retry_delay(7001),
        Some(std::time::Duration::from_secs(10))
    );
    assert_eq!(api_retry_delay(5000), None);
}
