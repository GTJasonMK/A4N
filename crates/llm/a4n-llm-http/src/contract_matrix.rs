use a4n_domain::llm::{LLM_PROVIDER_CAPABILITIES, LlmProtocolKind};
use a4n_llm_provider_contract::{ChatRequest, ModelCapabilities, collect_chat_stream};
use a4n_test_support::{
    model_contract::assert_case,
    provider_http::{LocalProvider, ProviderResponse},
};
use serde_json::{Value, json};

mod tools;

fn schema() -> Value {
    json!({"type":"object","required":["id","exit_state"],"additionalProperties":false,"properties":{"id":{"type":"integer","minimum":1},"exit_state":{"type":"object"}}})
}
fn request() -> ChatRequest {
    ChatRequest::system_user(
        "editable creative direction",
        "actual source input",
        Some(0.2),
        Some(256),
    )
    .with_json_schema_response_format("matrix", schema(), false)
    .unwrap()
}

fn events(protocol: LlmProtocolKind, scenario: &str) -> Vec<u8> {
    let content = if scenario == "empty" {
        " \n\t "
    } else {
        r#"{"id":1,"exit_state":{}}"#
    };
    events_with_content(protocol, scenario, content)
}

fn events_with_content(protocol: LlmProtocolKind, scenario: &str, content: &str) -> Vec<u8> {
    let finish = match scenario {
        "truncated" => "length",
        "refusal" => "content_filter",
        _ => "stop",
    };
    let mut events = match protocol {
        LlmProtocolKind::OpenAiChatCompletions => vec![
            json!({"choices":[{"index":0,"delta":{"content":content},"finish_reason":null}]}),
            json!({"choices":[{"index":0,"delta":{},"finish_reason":finish}],"usage":{"prompt_tokens":2,"completion_tokens":1,"total_tokens":3}}),
        ],
        LlmProtocolKind::OpenAiResponses => vec![
            json!({"type":"response.output_text.delta","delta":content}),
            json!({"type":"response.completed","response":{"status":if finish=="stop" {"completed"} else {"incomplete"},"incomplete_details":{"reason":if scenario=="truncated" {"max_output_tokens"} else {"content_filter"}},"output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":content}]}],"usage":{"input_tokens":2,"output_tokens":1,"total_tokens":3}}}),
        ],
        LlmProtocolKind::AnthropicMessages => vec![
            json!({"type":"message_start","message":{"usage":{"input_tokens":2,"output_tokens":0}}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":content}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"message_delta","delta":{"stop_reason":if scenario=="truncated" {"max_tokens"} else if scenario=="refusal" {"refusal"} else {"end_turn"}},"usage":{"output_tokens":1}}),
            json!({"type":"message_stop"}),
        ],
        LlmProtocolKind::GeminiGenerateContent => vec![
            json!({"candidates":[{"content":{"role":"model","parts":[{"text":content}]},"finishReason":if scenario=="truncated" {"MAX_TOKENS"} else if scenario=="refusal" {"SAFETY"} else {"STOP"}}],"usageMetadata":{"promptTokenCount":2,"candidatesTokenCount":1,"totalTokenCount":3}}),
        ],
    };
    if scenario == "missing-finish" {
        match protocol {
            LlmProtocolKind::OpenAiChatCompletions | LlmProtocolKind::OpenAiResponses => {
                events.pop();
            }
            LlmProtocolKind::AnthropicMessages => {
                events.truncate(4);
            }
            LlmProtocolKind::GeminiGenerateContent => {
                events[0]["candidates"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("finishReason");
            }
        }
    }
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect::<String>()
        .into_bytes()
}

#[tokio::test]
#[ignore = "run through the model runner with exported business schemas"]
async fn model_contract_matrix_business_schema_wire() {
    let path = std::env::var("A4N_MODEL_CONTRACT_CASES").unwrap();
    let document: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let call_id = document["call_id"].as_str().unwrap();
    let schema = &document["schema"];
    let output = document["valid"].to_string();
    for preset in LLM_PROVIDER_CAPABILITIES {
        let protocol = preset.protocol;
        let content = output.clone();
        let server = LocalProvider::start(move |_| {
            ProviderResponse::bytes(
                "text/event-stream",
                events_with_content(protocol, "valid", &content),
            )
        })
        .await;
        let request = ChatRequest::system_user("业务创作要求", "当前任务输入", None, Some(256));
        let request = if document["valid"].is_object() {
            request.with_json_schema_response_format(call_id, schema.clone(), false)
        } else {
            request.with_prompt_output_contract(call_id, schema.clone())
        }
        .unwrap();
        let validator = request.output_validator().unwrap().unwrap();
        let response =
            collect_chat_stream(provider(preset.id, &server.base_url, 2).as_ref(), request)
                .await
                .unwrap();
        validator
            .validate(
                &a4n_domain::json_contract::parse_strict_json_value(&response.message.content)
                    .unwrap(),
            )
            .unwrap();
        let captured = server.requests();
        assert_eq!(captured.len(), 1);
        let body: Value = serde_json::from_slice(&captured[0].body).unwrap();
        let rendered = body.to_string();
        assert!(rendered.contains("A4N_OUTPUT_CONTRACT"));
        let native = match protocol {
            LlmProtocolKind::OpenAiChatCompletions => {
                body.pointer("/response_format/json_schema/schema")
            }
            LlmProtocolKind::OpenAiResponses => body.pointer("/text/format/schema"),
            LlmProtocolKind::GeminiGenerateContent => {
                body.pointer("/generationConfig/responseJsonSchema")
            }
            LlmProtocolKind::AnthropicMessages => None,
        };
        if let Some(native) = native {
            assert_eq!(native, schema);
        }
        assert_case(
            call_id,
            preset.id,
            "business-schema",
            "actual-wire-roundtrip",
            json!(document["valid"]),
            json!(
                a4n_domain::json_contract::parse_strict_json_value(&response.message.content)
                    .unwrap()
            ),
            json!({"requests":captured.len(),"body_sha256":a4n_domain::source_text_hash(&rendered)}),
        );
    }
}

fn provider(
    kind: &str,
    base: &str,
    timeout: u64,
) -> Box<dyn a4n_llm_provider_contract::LlmProvider> {
    super::pick_llm_provider(
        kind,
        base.into(),
        "matrix-model".into(),
        Some("matrix-key".into()),
        timeout,
        None,
        ModelCapabilities {
            max_context_tokens: 100_000,
            max_output_tokens: 1024,
        },
    )
    .unwrap()
}

#[tokio::test]
async fn model_contract_matrix_llm_all_presets_use_actual_protocol_and_contract() {
    for preset in LLM_PROVIDER_CAPABILITIES {
        let protocol = preset.protocol;
        let server = LocalProvider::start(move |_| {
            ProviderResponse::bytes("text/event-stream", events(protocol, "valid"))
        })
        .await;
        let response =
            collect_chat_stream(provider(preset.id, &server.base_url, 2).as_ref(), request())
                .await
                .unwrap_or_else(|error| panic!("{}: {error}", preset.id));
        request()
            .validate_output_value(&serde_json::from_str(&response.message.content).unwrap())
            .unwrap();
        let captured = server.requests();
        assert_eq!(captured.len(), 1);
        let body: Value = serde_json::from_slice(&captured[0].body).unwrap();
        let text = body.to_string();
        assert!(
            text.contains("editable creative direction")
                && text.contains("actual source input")
                && text.contains("A4N_OUTPUT_CONTRACT")
        );
        assert!(text.contains("exit_state") && text.contains("integer"));
        let actual = match protocol {
            LlmProtocolKind::OpenAiChatCompletions => {
                assert!(captured[0].uri.ends_with("/chat/completions"));
                assert_eq!(body["max_tokens"], 256);
                assert_eq!(body["stream"], true);
                assert_eq!(captured[0].headers["authorization"], "Bearer matrix-key");
                if preset.id == "openai" {
                    assert_eq!(body["response_format"]["json_schema"]["schema"], schema());
                } else {
                    assert_eq!(body["response_format"]["type"], "json_object");
                }
                "chat"
            }
            LlmProtocolKind::OpenAiResponses => {
                assert!(captured[0].uri.ends_with("/responses"));
                assert_eq!(body["max_output_tokens"], 256);
                assert_eq!(body["text"]["format"]["schema"], schema());
                assert_eq!(captured[0].headers["authorization"], "Bearer matrix-key");
                "responses"
            }
            LlmProtocolKind::AnthropicMessages => {
                assert_eq!(captured[0].uri, "/v1/messages");
                assert_eq!(body["max_tokens"], 256);
                assert_eq!(captured[0].headers["x-api-key"], "matrix-key");
                "anthropic"
            }
            LlmProtocolKind::GeminiGenerateContent => {
                assert!(
                    captured[0]
                        .uri
                        .contains("matrix-model:streamGenerateContent?alt=sse")
                );
                assert_eq!(body["generationConfig"]["maxOutputTokens"], 256);
                assert_eq!(body["generationConfig"]["responseJsonSchema"], schema());
                assert_eq!(captured[0].headers["x-goog-api-key"], "matrix-key");
                "gemini"
            }
        };
        assert_case(
            "llm.transport",
            preset.id,
            "json",
            "actual-protocol-contract-budget",
            json!(true),
            json!(response.message.content.contains("exit_state")),
            json!({"protocol":actual,"uri":captured[0].uri,"requests":captured.len(),"body_sha256":a4n_domain::source_text_hash(&text)}),
        );
    }
}

#[tokio::test]
async fn model_contract_matrix_llm_all_presets_reject_failures_without_blind_retry() {
    for preset in LLM_PROVIDER_CAPABILITIES {
        for scenario in [
            "auth",
            "rate-limit",
            "server",
            "malformed-sse",
            "missing-finish",
            "truncated",
            "refusal",
            "empty",
            "disconnect",
        ] {
            let protocol = preset.protocol;
            let server = LocalProvider::start(move |_| {
                let status = match scenario {
                    "auth" => 401,
                    "rate-limit" => 429,
                    "server" => 503,
                    _ => 200,
                };
                if status != 200 {
                    return ProviderResponse::json(
                        status,
                        json!({"error":{"message":"injected fault"}}),
                    );
                }
                let mut response = ProviderResponse::bytes(
                    "text/event-stream",
                    if scenario == "malformed-sse" {
                        b"data: {invalid}\n\n".to_vec()
                    } else {
                        events(protocol, scenario)
                    },
                );
                response.stream_error = scenario == "disconnect";
                response
            })
            .await;
            let result =
                collect_chat_stream(provider(preset.id, &server.base_url, 2).as_ref(), request())
                    .await;
            assert_case(
                "llm.transport",
                preset.id,
                "json",
                scenario,
                json!(true),
                json!(result.is_err()),
                json!({"requests":server.requests().len(),"error":result.err().map(|error|error.to_string())}),
            );
            assert_eq!(
                server.requests().len(),
                1,
                "adapter must not repeat a billed request"
            );
        }
    }
}

#[tokio::test]
async fn model_contract_matrix_llm_deadlines_are_bounded() {
    for preset in LLM_PROVIDER_CAPABILITIES {
        let server = LocalProvider::start(|_| {
            let mut response = ProviderResponse::json(200, json!({}));
            response.delay_ms = 3000;
            response
        })
        .await;
        let start = std::time::Instant::now();
        let result =
            collect_chat_stream(provider(preset.id, &server.base_url, 1).as_ref(), request()).await;
        assert_case(
            "llm.transport",
            preset.id,
            "json",
            "bounded-timeout",
            json!(true),
            json!(result.is_err() && start.elapsed().as_millis() < 2500),
            json!({"elapsed_ms":start.elapsed().as_millis(),"requests":server.requests().len()}),
        );
    }
}
