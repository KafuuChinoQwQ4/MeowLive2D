use meowlive_protocol::resources::{DesktopResourceOperation, DesktopResourceResult};

#[test]
fn every_obs_operation_rejects_unknown_fields_before_execution() {
    for mut operation in [
        serde_json::json!({"type":"status"}),
        serde_json::json!({"type":"set_scene", "scene_name":"直播"}),
        serde_json::json!({"type":"start_recording"}),
        serde_json::json!({"type":"stop_recording"}),
    ] {
        operation["password"] = serde_json::json!("unexpected");
        assert!(
            serde_json::from_value::<meowlive_protocol::obs::ObsOperation>(operation.clone())
                .is_err(),
            "unexpected field accepted: {operation}"
        );
    }
}

#[test]
fn resource_channel_accepts_obs_operations_without_raw_rpc_or_secrets() {
    for operation in [
        serde_json::json!({"type":"status"}),
        serde_json::json!({"type":"set_scene", "scene_name":"直播"}),
        serde_json::json!({"type":"start_recording"}),
        serde_json::json!({"type":"stop_recording"}),
    ] {
        let value = serde_json::json!({"type":"obs", "operation":operation});
        let parsed = serde_json::from_value::<DesktopResourceOperation>(value.clone());
        assert!(parsed.is_ok(), "OBS operation rejected: {parsed:?}");
        assert_eq!(serde_json::to_value(parsed.unwrap()).unwrap(), value);
    }
    for operation in [
        serde_json::json!({"type":"start_streaming"}),
        serde_json::json!({"type":"status", "password":"secret"}),
        serde_json::json!({"type":"request", "request_type":"StartStream"}),
    ] {
        assert!(
            serde_json::from_value::<DesktopResourceOperation>(
                serde_json::json!({"type":"obs", "operation":operation})
            )
            .is_err()
        );
    }
}

#[test]
fn obs_snapshot_round_trips_on_resource_channel() {
    let value = serde_json::json!({"type":"obs","snapshot":{"connected":true,"recording":false,"current_scene":"直播","scenes":["直播", "待机"]}});
    let parsed = serde_json::from_value::<DesktopResourceResult>(value.clone());
    assert!(parsed.is_ok(), "OBS snapshot rejected: {parsed:?}");
    assert_eq!(serde_json::to_value(parsed.unwrap()).unwrap(), value);
}

#[test]
fn obs_settings_cross_resource_channel_without_exposing_password_in_debug() {
    let value = serde_json::json!({"type":"save_obs_settings","settings":{
        "enabled":true,"websocket_url":"ws://127.0.0.1:4455",
        "password":"private-test-password","clear_password":false
    }});
    let parsed = serde_json::from_value::<DesktopResourceOperation>(value.clone());
    assert!(
        parsed.is_ok(),
        "OBS settings operation rejected: {parsed:?}"
    );
    let parsed = parsed.unwrap();
    assert!(!format!("{parsed:?}").contains("private-test-password"));
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    let snapshot = serde_json::json!({"type":"obs_settings","settings":{
        "enabled":true,"websocket_url":"ws://127.0.0.1:4455",
        "password_configured":true,"storage_available":true
    }});
    assert!(serde_json::from_value::<DesktopResourceResult>(snapshot).is_ok());
}
