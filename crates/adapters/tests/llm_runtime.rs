mod llm_support;

use axum::{
    Json, Router,
    body::{Body, Bytes},
    response::Response,
    routing::post,
};
use meowlive_adapters::llm::multi_provider::{ApiFormat, MultiProvider};
use meowlive_application::ports::{
    llm::LanguageModel,
    llm_runtime::{
        ModelEvent, ModelObserver, ModelOptions, ToolDefinition, ToolExchange, ToolResult,
    },
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Events(Mutex<Vec<ModelEvent>>);
impl ModelObserver for Events {
    fn on_event(&self, event: ModelEvent) {
        self.0.lock().unwrap().push(event);
    }
}
fn options(stream: bool) -> ModelOptions {
    ModelOptions {
        stream,
        cache_enabled: true,
        tools: vec![ToolDefinition {
            name: "read_time".into(),
            description: "Read current time".into(),
            parameters_json: json!({"type":"object","properties":{},"additionalProperties":false})
                .to_string(),
        }],
        ..Default::default()
    }
}
fn formats() -> [ApiFormat; 4] {
    [
        ApiFormat::OpenaiChat,
        ApiFormat::OpenaiResponses,
        ApiFormat::AnthropicMessages,
        ApiFormat::GeminiGenerateContent,
    ]
}
fn answer() -> String {
    json!({"reply_to":[],"text":"你好猫咪","topic":null}).to_string()
}
fn usage(format: ApiFormat) -> Value {
    match format {
        ApiFormat::OpenaiChat => {
            json!({"prompt_tokens":100,"completion_tokens":12,"prompt_tokens_details":{"cached_tokens":30},"completion_tokens_details":{"reasoning_tokens":2}})
        }
        ApiFormat::OpenaiResponses => {
            json!({"input_tokens":100,"output_tokens":12,"input_tokens_details":{"cached_tokens":30,"cache_write_tokens":10},"output_tokens_details":{"reasoning_tokens":2}})
        }
        ApiFormat::AnthropicMessages => {
            json!({"input_tokens":40,"cache_read_input_tokens":30,"cache_creation_input_tokens":30,"output_tokens":12})
        }
        ApiFormat::GeminiGenerateContent => {
            json!({"promptTokenCount":100,"candidatesTokenCount":10,"thoughtsTokenCount":2,"cachedContentTokenCount":30})
        }
    }
}
fn response(format: ApiFormat, tool: bool) -> Value {
    let mut v = match format {
        ApiFormat::OpenaiChat => {
            json!({"choices":[{"index":0,"finish_reason":if tool {"tool_calls"} else {"stop"},"message":if tool {json!({"role":"assistant","content":null,"reasoning_content":"private-thinking","tool_calls":[{"id":"call-1","type":"function","function":{"name":"read_time","arguments":"{}"}}]})} else {json!({"role":"assistant","content":answer()})}}]})
        }
        ApiFormat::OpenaiResponses => {
            json!({"status":"completed","output":if tool {json!([{"type":"reasoning","id":"r1","summary":[],"encrypted_content":"private-signature"},{"type":"function_call","id":"fc1","call_id":"call-1","name":"read_time","arguments":"{}","status":"completed"}])} else {json!([{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":answer()}]}])}})
        }
        ApiFormat::AnthropicMessages => {
            json!({"type":"message","role":"assistant","stop_reason":if tool {"tool_use"} else {"end_turn"},"content":if tool {json!([{"type":"thinking","thinking":"private-thinking","signature":"private-signature"},{"type":"tool_use","id":"call-1","name":"read_time","input":{}}])} else {json!([{"type":"text","text":answer()}])}})
        }
        ApiFormat::GeminiGenerateContent => {
            json!({"candidates":[{"index":0,"finishReason":"STOP","content":{"role":"model","parts":if tool {json!([{"functionCall":{"id":"call-1","name":"read_time","args":{}},"thoughtSignature":"private-signature"}])} else {json!([{"text":answer()}])}}}]})
        }
    };
    let key = if format == ApiFormat::GeminiGenerateContent {
        "usageMetadata"
    } else {
        "usage"
    };
    v[key] = usage(format);
    v
}
async fn fixture(
    bodies: Vec<String>,
    stream: bool,
) -> (
    MultiProvider,
    Arc<Mutex<Vec<Value>>>,
    tokio::task::JoinHandle<()>,
) {
    fixture_for(ApiFormat::OpenaiChat, bodies, stream).await
}
async fn fixture_for(
    format: ApiFormat,
    bodies: Vec<String>,
    stream: bool,
) -> (
    MultiProvider,
    Arc<Mutex<Vec<Value>>>,
    tokio::task::JoinHandle<()>,
) {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let seen = captured.clone();
    let queue = Arc::new(Mutex::new(VecDeque::from(bodies)));
    let (url, task) =
        llm_support::server(Router::new().fallback(post(move |Json(body): Json<Value>| {
            let seen = seen.clone();
            let queue = queue.clone();
            async move {
                seen.lock().unwrap().push(body);
                let data = queue
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("unexpected request");
                let chunks = data
                    .as_bytes()
                    .chunks(7)
                    .map(|b| Ok::<_, std::io::Error>(Bytes::copy_from_slice(b)))
                    .collect::<Vec<_>>();
                Response::builder()
                    .header(
                        "content-type",
                        if stream {
                            "text/event-stream"
                        } else {
                            "application/json"
                        },
                    )
                    .body(Body::from_stream(futures_util::stream::iter(chunks)))
                    .unwrap()
            }
        })))
        .await;
    (
        MultiProvider::new(llm_support::config(url), format).unwrap(),
        captured,
        task,
    )
}
fn frame(value: Value) -> String {
    format!("data: {value}\r\n\r\n")
}
fn stream_answer(format: ApiFormat, finish: bool) -> String {
    match format {
        ApiFormat::OpenaiChat => {
            let mut result = frame(
                json!({"choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}),
            );
            for part in answer().chars().collect::<Vec<_>>().chunks(3) {
                result += &frame(
                    json!({"choices":[{"index":0,"delta":{"content":part.iter().collect::<String>()},"finish_reason":null}]}),
                );
            }
            result += &frame(json!({"choices":[],"usage":usage(format)}));
            if finish {
                result +=
                    &frame(json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}));
                result += "data: [DONE]\r\n\r\n";
            }
            result
        }
        ApiFormat::OpenaiResponses => {
            let mut result = frame(
                json!({"type":"response.created","response":{"status":"in_progress","output":[]}}),
            );
            result += &frame(json!({"type":"response.output_text.delta","delta":answer()}));
            result += &frame(
                json!({"type":if finish {"response.completed"} else {"response.incomplete"},"response":(if finish { response(format,false) } else { let mut v=response(format,false);v["status"]=json!("incomplete");v })}),
            );
            result
        }
        ApiFormat::AnthropicMessages => {
            let mut result = frame(
                json!({"type":"message_start","message":{"type":"message","role":"assistant","content":[],"usage":usage(format)}}),
            );
            result += &frame(
                json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
            );
            result += &frame(
                json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":answer()}}),
            );
            result += &frame(json!({"type":"content_block_stop","index":0}));
            result += &frame(
                json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":12}}),
            );
            if finish {
                result += &frame(json!({"type":"message_stop"}));
            }
            result
        }
        ApiFormat::GeminiGenerateContent => {
            let mut v = response(format, false);
            if !finish {
                v["candidates"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("finishReason");
            }
            frame(v)
        }
    }
}

#[tokio::test]
async fn all_protocols_normalize_usage_and_keep_missing_usage_unknown() {
    for format in formats() {
        let mut no_usage = response(format, false);
        no_usage
            .as_object_mut()
            .unwrap()
            .remove(if format == ApiFormat::GeminiGenerateContent {
                "usageMetadata"
            } else {
                "usage"
            });
        let (adapter, _, task) = fixture_for(
            format,
            vec![response(format, false).to_string(), no_usage.to_string()],
            false,
        )
        .await;
        let turn = adapter
            .turn(llm_support::request(vec![]), options(false))
            .await
            .unwrap();
        assert_eq!(turn.usage.input_tokens, Some(100), "{format:?}");
        assert_eq!(turn.usage.output_tokens, Some(12), "{format:?}");
        assert_eq!(turn.usage.cache_read_tokens, Some(30));
        assert_eq!(
            adapter
                .turn(llm_support::request(vec![]), options(false))
                .await
                .unwrap()
                .usage
                .input_tokens,
            None
        );
        task.abort();
    }
}
#[tokio::test]
async fn all_protocols_round_trip_native_tools_and_signatures() {
    for format in formats() {
        let (adapter, seen, task) = fixture_for(
            format,
            vec![
                response(format, true).to_string(),
                response(format, false).to_string(),
            ],
            false,
        )
        .await;
        let turn = adapter
            .turn(llm_support::request(vec![]), options(false))
            .await
            .unwrap();
        assert!(turn.decision.is_none());
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].name, "read_time");
        let mut opts = options(false);
        opts.exchanges.push(ToolExchange {
            continuation: turn.continuation.unwrap(),
            results: vec![ToolResult {
                call_id: turn.tool_calls[0].id.clone(),
                name: "read_time".into(),
                content: "2026-09-22T12:00:00Z".into(),
                is_error: false,
            }],
        });
        assert_eq!(
            adapter
                .turn(llm_support::request(vec![]), opts)
                .await
                .unwrap()
                .decision
                .unwrap()
                .text
                .as_deref(),
            Some("你好猫咪")
        );
        let requests = seen.lock().unwrap();
        let body = &requests[1];
        assert!(body.to_string().contains("2026-09-22T12:00:00Z"));
        assert!(
            body.to_string()
                .contains(if format == ApiFormat::OpenaiChat {
                    "private-thinking"
                } else {
                    "private-signature"
                })
        );
        assert!(
            !requests[0]
                .to_string()
                .contains("Do not execute commands, call tools")
        );
        task.abort();
    }
}
#[tokio::test]
async fn split_sse_frames_finish_and_emit_only_public_progress_and_usage() {
    for format in formats() {
        let (adapter, _, task) = fixture_for(format, vec![stream_answer(format, true)], true).await;
        let events = Arc::new(Events::default());
        let mut opts = options(true);
        opts.observer = Some(events.clone());
        let turn = adapter
            .turn(llm_support::request(vec![]), opts)
            .await
            .unwrap();
        assert_eq!(turn.decision.unwrap().text.as_deref(), Some("你好猫咪"));
        assert_eq!(turn.usage.input_tokens, Some(100));
        assert_eq!(turn.usage.output_tokens, Some(12));
        let log = events.0.lock().unwrap();
        assert_eq!(
            log.iter()
                .filter(|v| matches!(v, ModelEvent::FirstToken))
                .count(),
            1
        );
        assert!(log.iter().any(|v|matches!(v,ModelEvent::OutputProgress{characters} if *characters==answer().chars().count() as u64)));
        assert!(
            log.iter()
                .any(|v| matches!(v,ModelEvent::Usage(u) if u.input_tokens==Some(100)))
        );
        task.abort();
    }
}
#[tokio::test]
async fn unfinished_streams_are_errors_but_report_received_usage() {
    for format in formats() {
        let (adapter, _, task) =
            fixture_for(format, vec![stream_answer(format, false)], true).await;
        let events = Arc::new(Events::default());
        let mut opts = options(true);
        opts.observer = Some(events.clone());
        assert!(
            adapter
                .turn(llm_support::request(vec![]), opts)
                .await
                .is_err()
        );
        assert!(
            events
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|v| matches!(v,ModelEvent::Usage(u) if u.input_tokens==Some(100))),
            "{format:?}"
        );
        task.abort();
    }
}
#[tokio::test]
async fn cache_controls_are_opt_in_and_compatibility_endpoints_get_no_openai_extensions() {
    for format in formats() {
        let (adapter, seen, task) =
            fixture_for(format, vec![response(format, false).to_string(); 2], false).await;
        adapter
            .turn(llm_support::request(vec![]), options(false))
            .await
            .unwrap();
        let mut opts = options(false);
        opts.cache_enabled = false;
        adapter
            .turn(llm_support::request(vec![]), opts)
            .await
            .unwrap();
        let seen = seen.lock().unwrap();
        assert!(seen[0].get("prompt_cache_key").is_none());
        if format == ApiFormat::AnthropicMessages {
            assert!(seen[0].to_string().contains("cache_control"));
            assert!(!seen[1].to_string().contains("cache_control"));
        }
        task.abort();
    }
}
#[tokio::test]
async fn duplicate_unknown_or_invalid_tool_calls_are_rejected() {
    for mutation in ["duplicate", "unknown", "arguments"] {
        let mut body = response(ApiFormat::OpenaiChat, true);
        match mutation {
            "duplicate" => {
                let c = body["choices"][0]["message"]["tool_calls"][0].clone();
                body["choices"][0]["message"]["tool_calls"]
                    .as_array_mut()
                    .unwrap()
                    .push(c);
            }
            "unknown" => {
                body["choices"][0]["message"]["tool_calls"][0]["function"]["name"] =
                    json!("execute_shell")
            }
            _ => {
                body["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
                    json!("[]")
            }
        }
        let (adapter, _, task) = fixture(vec![body.to_string()], false).await;
        assert!(
            adapter
                .turn(llm_support::request(vec![]), options(false))
                .await
                .is_err()
        );
        task.abort();
    }
}

fn stream_tool(format: ApiFormat) -> String {
    match format {
        ApiFormat::OpenaiChat=>[
            frame(json!({"choices":[{"index":0,"delta":{"reasoning_content":"private-thinking","tool_calls":[{"index":0,"id":"call-1","type":"function","function":{"name":"read_time","arguments":"{"}}]},"finish_reason":null}]})),
            frame(json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"}"}}]},"finish_reason":null}]})),
            frame(json!({"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]})),
            frame(json!({"choices":[],"usage":usage(format)})),"data: [DONE]\n\n".into(),
        ].concat(),
        ApiFormat::OpenaiResponses=>[
            frame(json!({"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","id":"fc1","call_id":"call-1","name":"read_time","arguments":""}})),
            frame(json!({"type":"response.function_call_arguments.delta","item_id":"fc1","delta":"{"})),
            frame(json!({"type":"response.function_call_arguments.delta","item_id":"fc1","delta":"}"})),
            frame(json!({"type":"response.completed","response":response(format,true)})),
        ].concat(),
        ApiFormat::AnthropicMessages=>[
            frame(json!({"type":"message_start","message":{"type":"message","role":"assistant","content":[],"usage":usage(format)}})),
            frame(json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}})),
            frame(json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"private-thinking"}})),
            frame(json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"private-signature"}})),
            frame(json!({"type":"content_block_stop","index":0})),
            frame(json!({"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"call-1","name":"read_time","input":{}}})),
            frame(json!({"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{"}})),
            frame(json!({"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"}"}})),
            frame(json!({"type":"content_block_stop","index":1})),
            frame(json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":12}})),
            frame(json!({"type":"message_stop"})),
        ].concat(),
        ApiFormat::GeminiGenerateContent=>frame(response(format,true)),
    }
}

#[tokio::test]
async fn streamed_tool_arguments_and_private_signatures_round_trip_without_public_text() {
    for format in formats() {
        let (adapter, seen, task) = fixture_for(
            format,
            vec![stream_tool(format), stream_answer(format, true)],
            true,
        )
        .await;
        let events = Arc::new(Events::default());
        let mut opts = options(true);
        opts.observer = Some(events.clone());
        let turn = adapter
            .turn(llm_support::request(vec![]), opts.clone())
            .await
            .unwrap();
        assert_eq!(turn.tool_calls[0].arguments_json, "{}");
        assert!(
            !events
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|e| matches!(e, ModelEvent::OutputProgress { .. })),
            "{format:?}"
        );
        opts.exchanges.push(ToolExchange {
            continuation: turn.continuation.unwrap(),
            results: vec![ToolResult {
                call_id: turn.tool_calls[0].id.clone(),
                name: "read_time".into(),
                content: "upstream timeout".into(),
                is_error: true,
            }],
        });
        adapter
            .turn(llm_support::request(vec![]), opts)
            .await
            .unwrap();
        let body = &seen.lock().unwrap()[1];
        match format {
            ApiFormat::OpenaiChat => {
                assert_eq!(body["messages"][3]["role"], "assistant");
                assert_eq!(body["messages"][4]["tool_call_id"], "call-1");
            }
            ApiFormat::OpenaiResponses => {
                assert_eq!(body["input"][3]["encrypted_content"], "private-signature");
                assert_eq!(body["input"][5]["type"], "function_call_output");
                assert_eq!(body["input"][5]["call_id"], "call-1");
            }
            ApiFormat::AnthropicMessages => {
                assert_eq!(
                    body["messages"][1]["content"][0]["signature"],
                    "private-signature"
                );
                assert_eq!(body["messages"][2]["content"][0]["is_error"], true);
            }
            ApiFormat::GeminiGenerateContent => {
                assert_eq!(
                    body["contents"][1]["parts"][0]["thoughtSignature"],
                    "private-signature"
                );
                assert_eq!(
                    body["contents"][2]["parts"][0]["functionResponse"]["id"],
                    "call-1"
                );
                assert_eq!(
                    body["contents"][2]["parts"][0]["functionResponse"]["response"]["error"],
                    "upstream timeout"
                );
            }
        }
        task.abort();
    }
}

#[tokio::test]
async fn abnormal_finish_reasons_still_record_usage_in_nonstream_responses() {
    for format in formats() {
        let mut raw = response(format, false);
        match format {
            ApiFormat::OpenaiChat => raw["choices"][0]["finish_reason"] = json!("length"),
            ApiFormat::OpenaiResponses => raw["status"] = json!("incomplete"),
            ApiFormat::AnthropicMessages => raw["stop_reason"] = json!("max_tokens"),
            ApiFormat::GeminiGenerateContent => {
                raw["candidates"][0]["finishReason"] = json!("MAX_TOKENS")
            }
        }
        let (adapter, _, task) = fixture_for(format, vec![raw.to_string()], false).await;
        let events = Arc::new(Events::default());
        let mut opts = options(false);
        opts.observer = Some(events.clone());
        assert!(
            adapter
                .turn(llm_support::request(vec![]), opts)
                .await
                .is_err()
        );
        assert!(
            events
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|e| matches!(e,ModelEvent::Usage(u) if u.input_tokens==Some(100)))
        );
        task.abort();
    }
}

#[tokio::test]
async fn gemini_without_provider_call_ids_uses_only_local_correlation_ids() {
    let mut tool = response(ApiFormat::GeminiGenerateContent, true);
    tool["candidates"][0]["content"]["parts"][0]["functionCall"]
        .as_object_mut()
        .unwrap()
        .remove("id");
    let (adapter, seen, task) = fixture_for(
        ApiFormat::GeminiGenerateContent,
        vec![
            tool.to_string(),
            response(ApiFormat::GeminiGenerateContent, false).to_string(),
        ],
        false,
    )
    .await;
    let turn = adapter
        .turn(llm_support::request(vec![]), options(false))
        .await
        .unwrap();
    let mut opts = options(false);
    opts.exchanges.push(ToolExchange {
        continuation: turn.continuation.unwrap(),
        results: vec![ToolResult {
            call_id: turn.tool_calls[0].id.clone(),
            name: "read_time".into(),
            content: "time".into(),
            is_error: false,
        }],
    });
    adapter
        .turn(llm_support::request(vec![]), opts)
        .await
        .unwrap();
    assert!(
        seen.lock().unwrap()[1]["contents"][2]["parts"][0]["functionResponse"]
            .get("id")
            .is_none()
    );
    task.abort();
}

#[tokio::test]
async fn bad_exchange_ids_are_rejected_before_sending_the_followup_request() {
    for format in formats() {
        let (adapter, seen, task) =
            fixture_for(format, vec![response(format, true).to_string()], false).await;
        let turn = adapter
            .turn(llm_support::request(vec![]), options(false))
            .await
            .unwrap();
        let mut opts = options(false);
        opts.exchanges.push(ToolExchange {
            continuation: turn.continuation.unwrap(),
            results: vec![ToolResult {
                call_id: "unknown".into(),
                name: "read_time".into(),
                content: "time".into(),
                is_error: false,
            }],
        });
        assert!(
            adapter
                .turn(llm_support::request(vec![]), opts)
                .await
                .is_err()
        );
        assert_eq!(seen.lock().unwrap().len(), 1);
        task.abort();
    }
}

#[tokio::test]
async fn cancelling_an_inflight_stream_closes_transport_and_keeps_received_usage() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    struct NotifyEvents {
        events: Events,
        ready: tokio::sync::Notify,
    }
    impl ModelObserver for NotifyEvents {
        fn on_event(&self, event: ModelEvent) {
            let usage = matches!(&event,ModelEvent::Usage(u) if u.input_tokens==Some(100));
            self.events.on_event(event);
            if usage {
                self.ready.notify_one();
            }
        }
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(socket.read_u8().await.unwrap());
        }
        let headers = String::from_utf8(headers).unwrap();
        let length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|s| s.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").await.unwrap();
        let data = frame(json!({"choices":[],"usage":usage(ApiFormat::OpenaiChat)}));
        socket
            .write_all(format!("{:x}\r\n{data}\r\n", data.len()).as_bytes())
            .await
            .unwrap();
        let mut byte = [0];
        tokio::time::timeout(std::time::Duration::from_secs(2), socket.read(&mut byte))
            .await
            .unwrap()
            .unwrap()
    });
    let adapter = MultiProvider::new(llm_support::config(url), ApiFormat::OpenaiChat).unwrap();
    let observer = Arc::new(NotifyEvents {
        events: Events::default(),
        ready: tokio::sync::Notify::new(),
    });
    let mut opts = options(true);
    opts.observer = Some(observer.clone());
    let call = tokio::spawn(async move { adapter.turn(llm_support::request(vec![]), opts).await });
    tokio::time::timeout(std::time::Duration::from_secs(1), observer.ready.notified())
        .await
        .unwrap();
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    assert_eq!(server.await.unwrap(), 0);
    assert!(
        observer
            .events
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e,ModelEvent::Usage(u) if u.input_tokens==Some(100)))
    );
}

#[tokio::test]
async fn malformed_utf8_after_a_complete_usage_frame_keeps_that_usage() {
    let data = frame(json!({"choices":[],"usage":usage(ApiFormat::OpenaiChat)}));
    let mut bytes = data.into_bytes();
    bytes.extend_from_slice(b"data: \xff\n\n");
    let (url, task) = llm_support::server(Router::new().fallback(post(move || {
        let bytes = bytes.clone();
        async move {
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(Body::from(bytes))
                .unwrap()
        }
    })))
    .await;
    let adapter = MultiProvider::new(llm_support::config(url), ApiFormat::OpenaiChat).unwrap();
    let events = Arc::new(Events::default());
    let mut opts = options(true);
    opts.observer = Some(events.clone());
    assert!(
        adapter
            .turn(llm_support::request(vec![]), opts)
            .await
            .is_err()
    );
    assert!(
        events
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e,ModelEvent::Usage(u) if u.input_tokens==Some(100)))
    );
    task.abort();
}

#[tokio::test]
async fn final_round_disables_calls_without_losing_tools_or_continuation() {
    for format in formats() {
        let (adapter, seen, task) = fixture_for(
            format,
            vec![
                response(format, true).to_string(),
                response(format, false).to_string(),
                response(format, true).to_string(),
            ],
            false,
        )
        .await;
        let turn = adapter
            .turn(llm_support::request(vec![]), options(false))
            .await
            .unwrap();
        let mut opts = options(false);
        opts.tool_choice_none = true;
        opts.exchanges.push(ToolExchange {
            continuation: turn.continuation.unwrap(),
            results: vec![ToolResult {
                call_id: turn.tool_calls[0].id.clone(),
                name: "read_time".into(),
                content: "time".into(),
                is_error: false,
            }],
        });
        assert!(
            adapter
                .turn(llm_support::request(vec![]), opts.clone())
                .await
                .unwrap()
                .decision
                .is_some()
        );
        {
            let seen = seen.lock().unwrap();
            let body = &seen[1];
            assert_eq!(seen[0]["tools"], body["tools"]);
            match format {
                ApiFormat::OpenaiChat | ApiFormat::OpenaiResponses => {
                    assert_eq!(body["tool_choice"], "none")
                }
                ApiFormat::AnthropicMessages => {
                    assert_eq!(body["tool_choice"], json!({"type":"none"}))
                }
                ApiFormat::GeminiGenerateContent => {
                    assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "NONE")
                }
            }
        }
        assert!(
            adapter
                .turn(llm_support::request(vec![]), opts)
                .await
                .is_err()
        );
        task.abort();
    }
}

#[tokio::test]
async fn web_search_guidance_is_present_without_changing_with_environment() {
    let (adapter, seen, task) = fixture(
        vec![response(ApiFormat::OpenaiChat, false).to_string(); 2],
        false,
    )
    .await;
    let mut opts = options(false);
    opts.tools[0].name = "web_search".into();
    adapter
        .turn(llm_support::request(vec![]), opts.clone())
        .await
        .unwrap();
    opts.environment = Some("new time".into());
    adapter
        .turn(llm_support::request(vec![]), opts)
        .await
        .unwrap();
    let seen = seen.lock().unwrap();
    let system = seen[0]["messages"][0]["content"].as_str().unwrap();
    assert!(
        system.contains("unfamiliar terms")
            && system.contains("web_search")
            && system.contains("clarification")
    );
    assert_eq!(seen[0]["messages"][0], seen[1]["messages"][0]);
    task.abort();
}

#[tokio::test]
async fn idless_parallel_gemini_results_follow_the_original_call_order() {
    let mut raw = response(ApiFormat::GeminiGenerateContent, true);
    raw["candidates"][0]["content"]["parts"] = json!([
        {"functionCall":{"name":"read_time","args":{"zone":"east"}},"thoughtSignature":"signature"},
        {"functionCall":{"name":"read_time","args":{"zone":"west"}}}
    ]);
    let (adapter, seen, task) = fixture_for(
        ApiFormat::GeminiGenerateContent,
        vec![
            raw.to_string(),
            response(ApiFormat::GeminiGenerateContent, false).to_string(),
        ],
        false,
    )
    .await;
    let turn = adapter
        .turn(llm_support::request(vec![]), options(false))
        .await
        .unwrap();
    let mut opts = options(false);
    opts.exchanges.push(ToolExchange {
        continuation: turn.continuation.unwrap(),
        results: vec![
            ToolResult {
                call_id: turn.tool_calls[1].id.clone(),
                name: "read_time".into(),
                content: "west result".into(),
                is_error: false,
            },
            ToolResult {
                call_id: turn.tool_calls[0].id.clone(),
                name: "read_time".into(),
                content: "east result".into(),
                is_error: false,
            },
        ],
    });
    adapter
        .turn(llm_support::request(vec![]), opts)
        .await
        .unwrap();
    let seen = seen.lock().unwrap();
    let results = &seen[1]["contents"][2]["parts"];
    assert_eq!(
        results[0]["functionResponse"]["response"]["result"],
        "east result"
    );
    assert_eq!(
        results[1]["functionResponse"]["response"]["result"],
        "west result"
    );
    task.abort();
}

#[tokio::test]
async fn runtime_preserves_declared_and_streamed_size_limits() {
    for stream in [false, true] {
        let body = if stream {
            format!(
                "{}:{}\n\n",
                frame(json!({"choices":[],"usage":usage(ApiFormat::OpenaiChat)})),
                "x".repeat(9000)
            )
        } else {
            let mut raw = response(ApiFormat::OpenaiChat, false);
            raw["padding"] = json!("x".repeat(2000));
            raw.to_string()
        };
        let (url, task) = llm_support::server(Router::new().fallback(post(move || {
            let body = body.clone();
            async move {
                if stream {
                    let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(body))];
                    Response::builder()
                        .header("content-type", "text/event-stream")
                        .body(Body::from_stream(futures_util::stream::iter(chunks)))
                        .unwrap()
                } else {
                    Response::builder().body(Body::from(body)).unwrap()
                }
            }
        })))
        .await;
        let mut config = llm_support::config(url);
        config.max_response_bytes = 1024;
        let adapter = MultiProvider::new(config, ApiFormat::OpenaiChat).unwrap();
        let events = Arc::new(Events::default());
        let mut opts = options(stream);
        opts.observer = Some(events.clone());
        let error = adapter
            .turn(llm_support::request(vec![]), opts)
            .await
            .unwrap_err();
        assert!(error.message.contains("size limit"));
        if stream {
            assert!(
                events
                    .0
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|e| matches!(e,ModelEvent::Usage(u) if u.input_tokens==Some(100)))
            );
        }
        task.abort();
    }
}

#[tokio::test]
async fn a_stalled_sse_body_times_out_and_retains_received_usage() {
    use futures_util::StreamExt;
    let data = frame(json!({"choices":[],"usage":usage(ApiFormat::OpenaiChat)}));
    let (url, task) = llm_support::server(Router::new().fallback(post(move || {
        let data = data.clone();
        async move {
            let chunks =
                futures_util::stream::once(
                    async move { Ok::<_, std::io::Error>(Bytes::from(data)) },
                )
                .chain(futures_util::stream::pending());
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(Body::from_stream(chunks))
                .unwrap()
        }
    })))
    .await;
    let mut config = llm_support::config(url);
    config.timeout = std::time::Duration::from_secs(1);
    let adapter = MultiProvider::new(config, ApiFormat::OpenaiChat).unwrap();
    let events = Arc::new(Events::default());
    let mut opts = options(true);
    opts.observer = Some(events.clone());
    let error = adapter
        .turn(llm_support::request(vec![]), opts)
        .await
        .unwrap_err();
    assert!(error.retryable);
    assert!(error.message.contains("timed out"));
    assert!(
        events
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e,ModelEvent::Usage(u) if u.input_tokens==Some(100)))
    );
    task.abort();
}

#[tokio::test]
async fn a_finished_transport_cannot_make_partial_tool_arguments_valid() {
    for format in formats() {
        let data = match format {
            ApiFormat::OpenaiChat => {
                stream_tool(format).replace(r#""arguments":"}""#, r#""arguments":"""#)
            }
            ApiFormat::AnthropicMessages => {
                stream_tool(format).replace(r#""partial_json":"}""#, r#""partial_json":"""#)
            }
            ApiFormat::OpenaiResponses => {
                let mut v = response(format, true);
                v["output"][1]["arguments"] = json!("{");
                frame(json!({"type":"response.completed","response":v}))
            }
            ApiFormat::GeminiGenerateContent => {
                let mut v = response(format, true);
                v["candidates"][0]["content"]["parts"][0]["functionCall"]["args"] = json!("{");
                frame(v)
            }
        };
        let (adapter, _, task) = fixture_for(format, vec![data], true).await;
        assert!(
            adapter
                .turn(llm_support::request(vec![]), options(true))
                .await
                .is_err(),
            "{format:?}"
        );
        task.abort();
    }
}

#[tokio::test]
async fn http_errors_preserve_reported_usage_without_exposing_provider_body() {
    for format in formats() {
        let mut body = response(format, false);
        body["error"] = json!({"message":"private-upstream-details"});
        let (url, task) = llm_support::server(Router::new().fallback(post(move || {
            let body = body.clone();
            async move { (axum::http::StatusCode::TOO_MANY_REQUESTS, Json(body)) }
        })))
        .await;
        let adapter = MultiProvider::new(llm_support::config(url), format).unwrap();
        let events = Arc::new(Events::default());
        let mut opts = options(false);
        opts.observer = Some(events.clone());
        let error = adapter
            .turn(llm_support::request(vec![]), opts)
            .await
            .unwrap_err();
        assert!(error.retryable);
        assert!(!error.message.contains("private-upstream-details"));
        assert!(
            events
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|e| matches!(e,ModelEvent::Usage(u) if u.input_tokens==Some(100)))
        );
        task.abort();
    }
}

#[tokio::test]
async fn chat_rejects_conflicting_legacy_calls_audio_and_error_content() {
    for field in ["function_call", "audio", "error"] {
        let mut raw = response(ApiFormat::OpenaiChat, false);
        if field == "error" {
            raw[field] = json!({"message":"provider failure"});
        } else {
            raw["choices"][0]["message"][field] = json!({"unexpected":"content"});
        }
        let (adapter, _, task) = fixture(vec![raw.to_string()], false).await;
        assert!(
            adapter
                .turn(llm_support::request(vec![]), options(false))
                .await
                .is_err(),
            "{field}"
        );
        task.abort();
    }
}

#[tokio::test]
async fn deepseek_null_stream_roles_are_optional_but_other_roles_remain_invalid() {
    for role in [Value::Null, json!("user")] {
        let data=[
            frame(json!({"choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]})),
            frame(json!({"choices":[{"index":0,"delta":{"role":role,"content":answer()},"finish_reason":null}]})),
            frame(json!({"choices":[{"index":0,"delta":{"role":null,"content":""},"finish_reason":"stop"}]})),
            "data: [DONE]\n\n".into(),
        ].concat();
        let (adapter, _, task) = fixture(vec![data], true).await;
        let turn = adapter
            .turn(llm_support::request(vec![]), options(true))
            .await;
        if role.is_null() {
            assert_eq!(
                turn.unwrap().decision.unwrap().text.as_deref(),
                Some("你好猫咪")
            );
        } else {
            assert!(turn.is_err());
        }
        task.abort();
    }
}

#[tokio::test]
async fn deepseek_cache_hits_are_input_subsets_and_standard_fields_take_precedence() {
    for (standard, expected) in [(None, 30), (Some(40), 40)] {
        for stream in [false, true] {
            let mut metering = json!({"prompt_tokens":100,"completion_tokens":12,"prompt_cache_hit_tokens":30,"prompt_cache_miss_tokens":70});
            if let Some(value) = standard {
                metering["prompt_tokens_details"] = json!({"cached_tokens":value});
            }
            let data = if stream {
                [frame(json!({"choices":[{"index":0,"delta":{"content":answer()},"finish_reason":null}]})),frame(json!({"choices":[],"usage":metering})),frame(json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})),"data: [DONE]\n\n".into()].concat()
            } else {
                let mut value = response(ApiFormat::OpenaiChat, false);
                value["usage"] = metering;
                value.to_string()
            };
            let (adapter, _, task) = fixture(vec![data], stream).await;
            let events = Arc::new(Events::default());
            let mut opts = options(stream);
            opts.observer = Some(events.clone());
            let turn = adapter
                .turn(llm_support::request(vec![]), opts)
                .await
                .unwrap();
            assert_eq!(turn.usage.input_tokens, Some(100));
            assert_eq!(turn.usage.cache_read_tokens, Some(expected));
            assert_eq!(turn.usage.cache_write_tokens, None);
            assert!(events.0.lock().unwrap().iter().any(|e|matches!(e,ModelEvent::Usage(u) if u.input_tokens==Some(100)&&u.cache_read_tokens==Some(expected))));
            task.abort();
        }
    }
}

#[tokio::test]
async fn usage_observer_only_receives_changed_snapshots_before_complete_or_error() {
    for completed in [true, false] {
        let initial = usage(ApiFormat::AnthropicMessages);
        let mut data = frame(
            json!({"type":"message_start","message":{"type":"message","role":"assistant","content":[],"usage":initial}}),
        );
        data += &frame(
            json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        );
        for chunk in answer().chars() {
            data += &frame(json!({"type":"ping"}));
            data += &frame(
                json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":chunk.to_string()}}),
            );
        }
        data += &frame(json!({"type":"content_block_stop","index":0}));
        data += &frame(
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":15}}),
        );
        data += &frame(json!({"type":"ping"}));
        data += &frame(json!({"type":"message_delta","delta":{},"usage":{"output_tokens":15}}));
        data += &frame(if completed {
            json!({"type":"message_stop"})
        } else {
            json!({"type":"error","error":{"type":"overloaded_error"}})
        });
        let (adapter, _, task) = fixture_for(ApiFormat::AnthropicMessages, vec![data], true).await;
        let events = Arc::new(Events::default());
        let mut opts = options(true);
        opts.observer = Some(events.clone());
        assert_eq!(
            adapter
                .turn(llm_support::request(vec![]), opts)
                .await
                .is_ok(),
            completed
        );
        let events = events.0.lock().unwrap();
        let snapshots = events
            .iter()
            .filter_map(|event| {
                if let ModelEvent::Usage(usage) = event {
                    Some(usage.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].output_tokens, Some(12));
        assert_eq!(snapshots[1].output_tokens, Some(15));
        assert_eq!(snapshots[1].input_tokens, Some(100));
        task.abort();
    }
}

#[tokio::test]
async fn a_valid_500_character_answer_fits_the_default_budget_despite_sse_metadata() {
    let answer = json!({"reply_to":[],"text":"猫".repeat(500),"topic":null}).to_string();
    let mut data = String::new();
    for character in answer.chars() {
        data += &frame(
            json!({"id":"chatcmpl-test-completion","object":"chat.completion.chunk","created":1750000000,"model":"test-model","choices":[{"index":0,"delta":{"role":null,"content":character.to_string()},"finish_reason":null}]}),
        );
    }
    data += &frame(
        json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":usage(ApiFormat::OpenaiChat)}),
    );
    data += "data: [DONE]\n\n";
    assert!(data.len() > 65536 && data.len() < 8 * 65536);
    let (url, task) = llm_support::server(Router::new().fallback(post(move || {
        let data = data.clone();
        async move {
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(Body::from(data))
                .unwrap()
        }
    })))
    .await;
    let mut config = llm_support::config(url);
    config.max_response_bytes = 65536;
    let adapter = MultiProvider::new(config, ApiFormat::OpenaiChat).unwrap();
    let turn = adapter
        .turn(llm_support::request(vec![]), options(true))
        .await
        .unwrap();
    assert_eq!(turn.decision.unwrap().text.unwrap().chars().count(), 500);
    task.abort();
}

#[tokio::test]
async fn the_assembled_stream_result_keeps_the_original_response_budget() {
    let answer = json!({"reply_to":[],"text":"猫".repeat(500),"topic":null}).to_string();
    let data=[frame(json!({"choices":[{"index":0,"delta":{"content":answer},"finish_reason":"stop"}],"usage":usage(ApiFormat::OpenaiChat)})),"data: [DONE]\n\n".into()].concat();
    assert!(data.len() > 1024 && data.len() < 8192);
    let (url, task) = llm_support::server(Router::new().fallback(post(move || {
        let data = data.clone();
        async move {
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(Body::from(data))
                .unwrap()
        }
    })))
    .await;
    let mut config = llm_support::config(url);
    config.max_response_bytes = 1024;
    let adapter = MultiProvider::new(config, ApiFormat::OpenaiChat).unwrap();
    let events = Arc::new(Events::default());
    let mut opts = options(true);
    opts.observer = Some(events.clone());
    let error = adapter
        .turn(llm_support::request(vec![]), opts)
        .await
        .unwrap_err();
    assert!(error.message.contains("size limit"));
    assert!(
        events
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e,ModelEvent::Usage(u) if u.input_tokens==Some(100)))
    );
    task.abort();
}
