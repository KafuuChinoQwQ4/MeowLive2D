use meowlive_protocol::control::{ClientMessage, ServerCommand, SpeechRequest, SpeechStatus};
use meowlive_protocol::execution::{ExecutionReceipt, ExecutionStatus};

#[test]
fn control_hello_carries_pairing_identity() {
    let hello = ServerCommand::Hello {
        protocol_version: 1,
        session_id: "session-1".into(),
        bridge_id: "bridge-1".into(),
        generation: 3,
    };
    assert_eq!(
        serde_json::to_value(hello).unwrap(),
        serde_json::json!({
            "type":"hello", "protocol_version":1, "session_id":"session-1", "bridge_id":"bridge-1", "generation":3
        })
    );
}

#[test]
fn execution_receipt_uses_nested_tagged_wire_shape() {
    let message = ClientMessage::Receipt {
        receipt: ExecutionReceipt {
            utterance_id: "speech-1".into(),
            generation: 4,
            status: ExecutionStatus::Completed,
            error: None,
        },
    };
    assert_eq!(
        serde_json::to_value(message).unwrap(),
        serde_json::json!({
            "type":"receipt", "receipt":{"utterance_id":"speech-1","generation":4,"status":"completed","error":null}
        })
    );
    assert_eq!(
        serde_json::to_string(&SpeechStatus::Synthesizing).unwrap(),
        "\"synthesizing\""
    );
}

#[test]
fn speech_request_rejects_missing_required_voice() {
    assert!(serde_json::from_str::<SpeechRequest>(r#"{"text":"hello"}"#).is_err());
}
