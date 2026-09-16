use futures_util::{SinkExt, StreamExt};
use meowlive_desktop_runtime::obs::{self, ObsConfig};
use meowlive_protocol::obs::ObsOperation;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

async fn config_for(listener: &TcpListener, timeout_ms: u64) -> ObsConfig {
    ObsConfig {
        enabled: true,
        websocket_url: format!("ws://{}", listener.local_addr().unwrap()),
        timeout_ms,
        ..ObsConfig::default()
    }
}

async fn spawn_obs(
    recording: bool,
    current_scene: &'static str,
) -> (ObsConfig, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(
                json!({"op":0,"d":{"rpcVersion":1}}).to_string().into(),
            ))
            .await
            .unwrap();
        let identify: Value =
            serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(identify, json!({"op":1,"d":{"rpcVersion":1}}));
        socket
            .send(Message::Text(
                json!({"op":2,"d":{"negotiatedRpcVersion":1}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();

        let mut requests = Vec::new();
        let mut recording = recording;
        let mut current_scene = current_scene.to_owned();
        while let Some(Ok(message)) = socket.next().await {
            let request: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
            let data = &request["d"];
            let kind = data["requestType"].as_str().unwrap();
            requests.push(kind.to_owned());
            let response_data = match kind {
                "GetRecordStatus" => json!({"outputActive":recording}),
                "GetSceneList" => {
                    json!({"currentProgramSceneName":current_scene,"scenes":[{"sceneName":"直播"},{"sceneName":"待机"}]})
                }
                "SetCurrentProgramScene" => {
                    current_scene = data["requestData"]["sceneName"]
                        .as_str()
                        .unwrap()
                        .to_owned();
                    json!({})
                }
                "StartRecord" => {
                    recording = true;
                    json!({})
                }
                "StopRecord" => {
                    recording = false;
                    json!({})
                }
                other => panic!("unexpected request: {other}"),
            };
            socket.send(Message::Text(json!({"op":7,"d":{"requestType":kind,"requestId":data["requestId"],"requestStatus":{"result":true,"code":100},"responseData":response_data}}).to_string().into())).await.unwrap();
        }
        requests
    });
    (
        ObsConfig {
            enabled: true,
            websocket_url: format!("ws://{address}"),
            timeout_ms: 1_000,
            ..ObsConfig::default()
        },
        task,
    )
}

#[tokio::test]
async fn recording_write_returns_fresh_state_without_retrying_the_write() {
    let (config, server) = spawn_obs(false, "直播").await;
    let snapshot = obs::execute(&config, ObsOperation::StartRecording)
        .await
        .unwrap();
    assert!(snapshot.recording);
    assert_eq!(
        server.await.unwrap(),
        [
            "GetRecordStatus",
            "GetSceneList",
            "StartRecord",
            "GetRecordStatus",
            "GetSceneList"
        ]
    );
}

#[tokio::test]
async fn scene_write_returns_fresh_state() {
    let (config, server) = spawn_obs(false, "直播").await;
    let snapshot = obs::execute(
        &config,
        ObsOperation::SetScene {
            scene_name: "待机".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(snapshot.current_scene, "待机");
    assert_eq!(
        server.await.unwrap(),
        [
            "GetRecordStatus",
            "GetSceneList",
            "SetCurrentProgramScene",
            "GetRecordStatus",
            "GetSceneList"
        ]
    );
}

#[tokio::test]
async fn unknown_scene_is_rejected_before_any_write_request() {
    let (config, server) = spawn_obs(false, "直播").await;
    let error = obs::execute(
        &config,
        ObsOperation::SetScene {
            scene_name: "不存在".into(),
        },
    )
    .await
    .unwrap_err();
    assert!(error.contains("不存在"), "unexpected error: {error}");
    assert_eq!(server.await.unwrap(), ["GetRecordStatus", "GetSceneList"]);
}

#[tokio::test]
async fn ping_is_answered_while_waiting_for_hello() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 1_000).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Ping(vec![1, 2, 3].into()))
            .await
            .unwrap();
        assert_eq!(
            socket.next().await.unwrap().unwrap(),
            Message::Pong(vec![1, 2, 3].into())
        );
        socket
            .send(Message::Text(
                json!({"op":0,"d":{"rpcVersion":1}}).to_string().into(),
            ))
            .await
            .unwrap();
        let _ = socket.next().await.unwrap().unwrap();
        socket
            .send(Message::Text(
                json!({"op":2,"d":{"negotiatedRpcVersion":1}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        for (kind, response) in [
            ("GetRecordStatus", json!({"outputActive":false})),
            (
                "GetSceneList",
                json!({"currentProgramSceneName":"直播","scenes":[{"sceneName":"直播"}]}),
            ),
        ] {
            let request: Value =
                serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap())
                    .unwrap();
            assert_eq!(request["d"]["requestType"], kind);
            socket.send(Message::Text(json!({"op":7,"d":{"requestType":kind,"requestId":request["d"]["requestId"],"requestStatus":{"result":true,"code":100},"responseData":response}}).to_string().into())).await.unwrap();
        }
    });
    let snapshot = obs::execute(&config, ObsOperation::Status).await.unwrap();
    assert!(snapshot.connected);
    server.await.unwrap();
}

#[tokio::test]
async fn challenge_without_password_fails_before_identify() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = config_for(&listener, 1_000).await;
    config.password_env = "MEOWLIVE_OBS_TEST_PASSWORD_MUST_BE_UNSET".into();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket.send(Message::Text(json!({"op":0,"d":{"rpcVersion":1,"authentication":{"salt":"salt","challenge":"challenge"}}}).to_string().into())).await.unwrap();
        assert!(matches!(
            socket.next().await,
            None | Some(Ok(Message::Close(_))) | Some(Err(_))
        ));
    });
    let error = obs::execute(&config, ObsOperation::Status)
        .await
        .unwrap_err();
    assert!(
        error.contains("密码环境变量缺失"),
        "unexpected error: {error}"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn mismatched_response_id_is_rejected() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 1_000).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(
                json!({"op":0,"d":{"rpcVersion":1}}).to_string().into(),
            ))
            .await
            .unwrap();
        let _ = socket.next().await.unwrap().unwrap();
        socket
            .send(Message::Text(
                json!({"op":2,"d":{"negotiatedRpcVersion":1}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        let request: Value =
            serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        socket.send(Message::Text(json!({"op":7,"d":{"requestType":"GetRecordStatus","requestId":"wrong","requestStatus":{"result":true,"code":100},"responseData":{"outputActive":false}}}).to_string().into())).await.unwrap();
        request
    });
    let error = obs::execute(&config, ObsOperation::Status)
        .await
        .unwrap_err();
    assert!(error.contains("不匹配"), "unexpected error: {error}");
    server.await.unwrap();
}

#[tokio::test]
async fn rejected_request_returns_stable_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 1_000).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(
                json!({"op":0,"d":{"rpcVersion":1}}).to_string().into(),
            ))
            .await
            .unwrap();
        let _ = socket.next().await.unwrap().unwrap();
        socket
            .send(Message::Text(
                json!({"op":2,"d":{"negotiatedRpcVersion":1}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        let request: Value =
            serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        socket.send(Message::Text(json!({"op":7,"d":{"requestType":"GetRecordStatus","requestId":request["d"]["requestId"],"requestStatus":{"result":false,"code":500,"comment":"sensitive detail"}}}).to_string().into())).await.unwrap();
    });
    let error = obs::execute(&config, ObsOperation::Status)
        .await
        .unwrap_err();
    assert_eq!(error, "OBS 请求 GetRecordStatus 被拒绝（代码 500）");
    server.await.unwrap();
}

#[tokio::test]
async fn operation_timeout_bounds_a_stalled_server() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 100).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _socket = accept_async(stream).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    });
    let error = obs::execute(&config, ObsOperation::Status)
        .await
        .unwrap_err();
    assert_eq!(error, "OBS 操作超时");
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn oversized_response_frame_is_rejected() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 1_000).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        // The client may reject the header and close before this payload is flushed.
        let _ = socket
            .send(Message::Text("x".repeat(256 * 1024 + 1).into()))
            .await;
    });
    let error = obs::execute(&config, ObsOperation::Status)
        .await
        .unwrap_err();
    assert!(error.contains("帧超限"), "unexpected error: {error}");
    server.await.unwrap();
}

#[tokio::test]
async fn disabled_obs_returns_disconnected_without_connecting() {
    let config = ObsConfig {
        websocket_url: "ws://127.0.0.1:9".into(),
        ..ObsConfig::default()
    };
    let snapshot = obs::execute(&config, ObsOperation::StartRecording)
        .await
        .unwrap();
    assert_eq!(
        (
            snapshot.connected,
            snapshot.recording,
            snapshot.current_scene,
            snapshot.scenes
        ),
        (false, false, String::new(), Vec::new())
    );
}

#[tokio::test]
async fn stop_recording_returns_fresh_state_and_idempotent_start_does_not_write() {
    let (config, server) = spawn_obs(true, "直播").await;
    let snapshot = obs::execute(&config, ObsOperation::StopRecording)
        .await
        .unwrap();
    assert!(!snapshot.recording);
    assert_eq!(
        server.await.unwrap(),
        [
            "GetRecordStatus",
            "GetSceneList",
            "StopRecord",
            "GetRecordStatus",
            "GetSceneList"
        ]
    );

    let (config, server) = spawn_obs(true, "直播").await;
    let snapshot = obs::execute(&config, ObsOperation::StartRecording)
        .await
        .unwrap();
    assert!(snapshot.recording);
    assert_eq!(server.await.unwrap(), ["GetRecordStatus", "GetSceneList"]);
}

type Socket = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

async fn send_json(socket: &mut Socket, value: Value) {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .unwrap();
}

async fn receive_json(socket: &mut Socket) -> Value {
    serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap()
}

async fn identify(socket: &mut Socket) {
    send_json(
        socket,
        json!({"op":0,"d":{"obsWebSocketVersion":"5.6.3","rpcVersion":1}}),
    )
    .await;
    assert_eq!(receive_json(socket).await["op"], 1);
    send_json(socket, json!({"op":2,"d":{"negotiatedRpcVersion":1}})).await;
}

async fn reply(socket: &mut Socket, request: &Value, data: Value) {
    send_json(socket, json!({"op":7,"d":{"requestType":request["d"]["requestType"],"requestId":request["d"]["requestId"],"requestStatus":{"result":true,"code":100},"responseData":data}})).await;
}

async fn assert_disconnected(socket: &mut Socket) {
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap(),
        None | Some(Ok(Message::Close(_))) | Some(Err(_))
    ));
}

#[tokio::test]
async fn oversized_frame_header_is_rejected_without_waiting_for_payload() {
    use tokio::io::AsyncWriteExt;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 200).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let mut header = vec![0x81, 127];
        header.extend_from_slice(&(256_u64 * 1024 + 1).to_be_bytes());
        socket.get_mut().write_all(&header).await.unwrap();
        assert_disconnected(&mut socket).await;
    });
    let error = obs::execute(&config, ObsOperation::Status)
        .await
        .unwrap_err();
    server.await.unwrap();
    assert!(error.contains("帧超限"), "unexpected error: {error}");
}

#[tokio::test]
async fn fragmented_messages_cannot_bypass_the_total_message_limit() {
    use tokio_tungstenite::tungstenite::protocol::frame::{
        Frame,
        coding::{Data, OpCode},
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 1_000).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Frame(Frame::message(
                vec![b'x'; 128 * 1024],
                OpCode::Data(Data::Text),
                false,
            )))
            .await
            .unwrap();
        let _ = socket
            .send(Message::Frame(Frame::message(
                vec![b'x'; 128 * 1024 + 1],
                OpCode::Data(Data::Continue),
                true,
            )))
            .await;
    });
    let error = obs::execute(&config, ObsOperation::Status)
        .await
        .unwrap_err();
    server.await.unwrap();
    assert!(error.contains("帧超限"), "unexpected error: {error}");
}

#[tokio::test]
async fn timeout_covers_the_entire_operation_instead_of_restarting_per_request() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 200).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        identify(&mut socket).await;
        let request = receive_json(&mut socket).await;
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        reply(&mut socket, &request, json!({"outputActive":false})).await;
        let request = receive_json(&mut socket).await;
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        let response = json!({"op":7,"d":{"requestType":"GetSceneList","requestId":request["d"]["requestId"],"requestStatus":{"result":true,"code":100},"responseData":{"currentProgramSceneName":"直播","scenes":[{"sceneName":"直播"}]}}});
        let _ = socket
            .send(Message::Text(response.to_string().into()))
            .await;
    });
    let result = obs::execute(&config, ObsOperation::Status).await;
    server.await.unwrap();
    assert_eq!(result.unwrap_err(), "OBS 操作超时");
}

#[tokio::test]
async fn malformed_authentication_is_rejected_before_loading_password() {
    for authentication in [
        json!(null),
        json!({"salt":"","challenge":"challenge"}),
        json!({"salt":"salt","challenge":""}),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut config = config_for(&listener, 1_000).await;
        config.password_env = "MEOWLIVE_OBS_TEST_PASSWORD_MUST_BE_UNSET".into();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            send_json(
                &mut socket,
                json!({"op":0,"d":{"rpcVersion":1,"authentication":authentication}}),
            )
            .await;
            assert_disconnected(&mut socket).await;
        });
        let error = obs::execute(&config, ObsOperation::Status)
            .await
            .unwrap_err();
        server.await.unwrap();
        assert!(
            error.contains("鉴权") && error.contains("无效"),
            "unexpected error: {error}"
        );
    }
}

#[tokio::test]
async fn contradictory_success_status_does_not_issue_a_write() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 1_000).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        identify(&mut socket).await;
        let request = receive_json(&mut socket).await;
        send_json(&mut socket, json!({"op":7,"d":{"requestType":"GetRecordStatus","requestId":request["d"]["requestId"],"requestStatus":{"result":true,"code":500},"responseData":{"outputActive":false}}})).await;
        // Any follow-up request would use untrusted state to decide whether to write.
        assert_disconnected(&mut socket).await;
    });
    let error = obs::execute(&config, ObsOperation::StartRecording)
        .await
        .unwrap_err();
    server.await.unwrap();
    assert!(error.contains("状态无效"), "unexpected error: {error}");
}

#[tokio::test]
async fn malformed_events_fail_closed() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 200).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        identify(&mut socket).await;
        let _ = receive_json(&mut socket).await;
        send_json(&mut socket, json!({"op":5,"d":null})).await;
        assert_disconnected(&mut socket).await;
    });
    let error = obs::execute(&config, ObsOperation::Status)
        .await
        .unwrap_err();
    server.await.unwrap();
    assert!(error.contains("事件无效"), "unexpected error: {error}");
}

#[tokio::test]
async fn events_and_ping_during_request_do_not_replace_the_matching_response() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = config_for(&listener, 1_000).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        identify(&mut socket).await;
        let request = receive_json(&mut socket).await;
        send_json(&mut socket, json!({"op":5,"d":{"eventType":"RecordStateChanged","eventIntent":64,"eventData":{"outputActive":true}}})).await;
        socket
            .send(Message::Ping(vec![4, 5, 6].into()))
            .await
            .unwrap();
        assert_eq!(
            socket.next().await.unwrap().unwrap(),
            Message::Pong(vec![4, 5, 6].into())
        );
        reply(&mut socket, &request, json!({"outputActive":false})).await;
        let request = receive_json(&mut socket).await;
        reply(
            &mut socket,
            &request,
            json!({"currentProgramSceneName":"直播","scenes":[{"sceneName":"直播"}]}),
        )
        .await;
    });
    let snapshot = obs::execute(&config, ObsOperation::Status).await.unwrap();
    assert!(snapshot.connected);
    assert!(!snapshot.recording);
    assert_eq!(snapshot.scenes, ["直播"]);
    server.await.unwrap();
}

#[tokio::test]
async fn lost_or_rejected_write_ack_and_failed_readback_never_replay_recording() {
    for failure in ["reject", "disconnect", "timeout", "readback"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = config_for(&listener, 200).await;
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            identify(&mut socket).await;
            for (kind, data) in [
                ("GetRecordStatus", json!({"outputActive":false})),
                (
                    "GetSceneList",
                    json!({"currentProgramSceneName":"直播","scenes":[{"sceneName":"直播"}]}),
                ),
            ] {
                let request = receive_json(&mut socket).await;
                assert_eq!(request["d"]["requestType"], kind);
                reply(&mut socket, &request, data).await;
            }
            let write = receive_json(&mut socket).await;
            assert_eq!(write["d"]["requestType"], "StartRecord");
            match failure {
                "reject" => send_json(&mut socket, json!({"op":7,"d":{"requestType":"StartRecord","requestId":write["d"]["requestId"],"requestStatus":{"result":false,"code":500,"comment":"private output path"}}})).await,
                "disconnect" => { socket.close(None).await.unwrap(); }
                "timeout" => {}
                "readback" => {
                    reply(&mut socket, &write, json!({})).await;
                    let read = receive_json(&mut socket).await;
                    assert_eq!(read["d"]["requestType"], "GetRecordStatus");
                    reply(&mut socket, &read, json!({"outputActive":"invalid"})).await;
                }
                _ => unreachable!(),
            }
            assert_disconnected(&mut socket).await;
            // A fresh connection would also constitute a forbidden implicit replay.
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(250), listener.accept())
                    .await
                    .is_err()
            );
        });
        let error = obs::execute(&config, ObsOperation::StartRecording)
            .await
            .unwrap_err();
        assert!(!error.contains("private output path"));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn invalid_handshake_and_response_messages_are_rejected() {
    for (phase, message) in [
        (0, json!({"op":0,"d":{"rpcVersion":0}})),
        (0, json!({"op":0,"d":{"rpcVersion":"1"}})),
        (0, json!({"op":9,"d":{}})),
        (1, json!({"op":2,"d":{"negotiatedRpcVersion":2}})),
        (1, json!({"op":7,"d":{}})),
        (
            2,
            json!({"op":7,"d":{"requestId":"ID","requestType":"StartStream","requestStatus":{"result":true,"code":100}}}),
        ),
        (
            2,
            json!({"op":7,"d":{"requestId":"ID","requestType":"GetRecordStatus","requestStatus":{"result":"true","code":100}}}),
        ),
        (2, json!({"op":9,"d":{}})),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = config_for(&listener, 1_000).await;
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let mut message = message;
            if phase > 0 {
                send_json(&mut socket, json!({"op":0,"d":{"rpcVersion":1}})).await;
                let _ = receive_json(&mut socket).await;
            }
            if phase > 1 {
                send_json(&mut socket, json!({"op":2,"d":{"negotiatedRpcVersion":1}})).await;
                let request = receive_json(&mut socket).await;
                if message["d"]["requestId"] == "ID" {
                    message["d"]["requestId"] = request["d"]["requestId"].clone();
                }
            }
            send_json(&mut socket, message).await;
            assert_disconnected(&mut socket).await;
        });
        assert!(
            obs::execute(&config, ObsOperation::StartRecording)
                .await
                .is_err()
        );
        server.await.unwrap();
    }
}

#[tokio::test]
async fn invalid_scene_data_cannot_authorize_a_control_request() {
    for scenes in [
        json!([{"sceneName":""}]),
        json!([{"sceneName":"bad\nname"}]),
        json!([{"sceneName":"x".repeat(257)}]),
        json!([{"sceneName":42}]),
        json!(vec![json!({"sceneName":"直播"}); 257]),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = config_for(&listener, 1_000).await;
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            identify(&mut socket).await;
            let request = receive_json(&mut socket).await;
            reply(&mut socket, &request, json!({"outputActive":false})).await;
            let request = receive_json(&mut socket).await;
            reply(
                &mut socket,
                &request,
                json!({"currentProgramSceneName":"直播","scenes":scenes}),
            )
            .await;
            assert_disconnected(&mut socket).await;
        });
        let error = obs::execute(&config, ObsOperation::StartRecording)
            .await
            .unwrap_err();
        assert!(error.contains("场景"));
        server.await.unwrap();
    }
}

// A subprocess isolates the fixture environment; Rust 2024 forbids mutating the
// shared process environment safely while parallel Tokio tests are running.
#[tokio::test]
async fn authentication_fixture_child() {
    let Ok(address) = std::env::var("MEOWLIVE_OBS_FIXTURE_ADDRESS") else {
        return;
    };
    let config = ObsConfig {
        enabled: true,
        websocket_url: address,
        password_env: "MEOWLIVE_OBS_FIXTURE_PASSWORD".into(),
        timeout_ms: 1_000,
    };
    let result = obs::execute(&config, ObsOperation::Status).await;
    if std::env::var("MEOWLIVE_OBS_FIXTURE_EXPECT_ERROR").is_ok() {
        let error = result.unwrap_err();
        assert!(!error.contains("supersecretpassword"));
    } else {
        let snapshot = result.unwrap();
        assert!(snapshot.connected);
        assert!(!snapshot.recording);
        assert_eq!(snapshot.current_scene, "直播");
    }
}

#[tokio::test]
async fn authenticated_session_uses_challenge_response_and_rejects_empty_or_denied_password() {
    for mode in ["success", "empty", "denied"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("ws://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            send_json(&mut socket, json!({"op":0,"d":{"obsWebSocketVersion":"5.6.3","rpcVersion":1,"authentication":{"salt":"lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=","challenge":"+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY="}}})).await;
            if mode == "empty" {
                assert_disconnected(&mut socket).await;
                return;
            }
            let identify = receive_json(&mut socket).await;
            assert_eq!(identify["op"], 1);
            assert_eq!(
                identify["d"]["authentication"],
                "1Ct943GAT+6YQUUX47Ia/ncufilbe6+oD6lY+5kaCu4="
            );
            if mode == "denied" {
                socket.close(None).await.unwrap();
                assert_disconnected(&mut socket).await;
                return;
            }
            send_json(&mut socket, json!({"op":2,"d":{"negotiatedRpcVersion":1}})).await;
            let request = receive_json(&mut socket).await;
            reply(&mut socket, &request, json!({"outputActive":false})).await;
            let request = receive_json(&mut socket).await;
            reply(
                &mut socket,
                &request,
                json!({"currentProgramSceneName":"直播","scenes":[{"sceneName":"直播"}]}),
            )
            .await;
        });
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "authentication_fixture_child", "--nocapture"])
            .env("MEOWLIVE_OBS_FIXTURE_ADDRESS", address)
            .env(
                "MEOWLIVE_OBS_FIXTURE_PASSWORD",
                if mode == "empty" {
                    ""
                } else {
                    "supersecretpassword"
                },
            )
            .env_remove("MEOWLIVE_OBS_FIXTURE_EXPECT_ERROR")
            .kill_on_drop(true);
        if mode != "success" {
            command.env("MEOWLIVE_OBS_FIXTURE_EXPECT_ERROR", "1");
        }
        let output = tokio::time::timeout(std::time::Duration::from_secs(5), command.output())
            .await
            .unwrap()
            .unwrap();
        server.await.unwrap();
        assert!(
            output.status.success(),
            "authentication fixture {mode} failed: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}
