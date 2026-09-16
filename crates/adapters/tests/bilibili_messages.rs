mod bilibili_support;

use bilibili_support::{brotli_packet, notify_packet, packet, zlib_packet};
use meowlive_adapters::live::bilibili::protocol::{DecodeLimits, api_retry_delay, decode_events};
use meowlive_domain::event::EventKind;

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
    assert_eq!(
        events[1].kind,
        EventKind::Gift {
            name: "Cat".into(),
            count: 3
        }
    );
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
