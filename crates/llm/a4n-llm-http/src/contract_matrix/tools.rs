use super::*;

fn tool_schema() -> Value {
    json!({"type":"object","required":["query"],"properties":{"query":{"type":"string"}},"additionalProperties":false})
}

fn tool_events(protocol: LlmProtocolKind, scenario: &str) -> Vec<u8> {
    let args = match scenario {
        "malformed-arguments" => r#"{"query":"雨夜""#,
        "duplicate-keys" => r#"{"query":"雨夜","query":"覆盖"}"#,
        _ => r#"{"query":"雨夜"}"#,
    };
    let name = if scenario == "missing-name" {
        ""
    } else {
        "search"
    };
    let mut rows = match protocol {
        LlmProtocolKind::OpenAiChatCompletions => {
            let split = args.find(':').unwrap() + 1;
            vec![
                json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call-1","type":"function","function":{"name":name,"arguments":&args[..split]}}]},"finish_reason":null}]}),
                json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":&args[split..]}}]},"finish_reason":null}]}),
                json!({"choices":[{"index":0,"delta":{},"finish_reason":if scenario=="truncated" {"length"} else {"tool_calls"}}]}),
            ]
        }
        LlmProtocolKind::OpenAiResponses => vec![
            json!({"type":"response.function_call_arguments.delta","delta":args}),
            json!({"type":"response.completed","response":{"status":"completed","output":[{"type":"function_call","call_id":"call-1","name":name,"arguments":args}]}}),
        ],
        LlmProtocolKind::AnthropicMessages => vec![
            json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"call-1","name":name,"input":{}}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":args}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"message_delta","delta":{"stop_reason":if scenario=="truncated" {"max_tokens"} else {"tool_use"}},"usage":{"output_tokens":1}}),
            json!({"type":"message_stop"}),
        ],
        LlmProtocolKind::GeminiGenerateContent => {
            let mut event = json!({"candidates":[{"content":{"role":"model","parts":[{"functionCall":{"id":"call-1","name":name,"args":"RAW_ARGUMENTS"}}]},"finishReason":if scenario=="truncated" {"MAX_TOKENS"} else {"STOP"}}]});
            if scenario == "missing-finish" {
                event["candidates"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("finishReason");
            }
            return format!(
                "data: {}\n\n",
                event.to_string().replace("\"RAW_ARGUMENTS\"", args)
            )
            .into_bytes();
        }
    };
    if scenario == "missing-finish" {
        match protocol {
            LlmProtocolKind::AnthropicMessages => rows.truncate(3),
            _ => {
                rows.pop();
            }
        }
    }
    if scenario == "truncated" && protocol == LlmProtocolKind::OpenAiResponses {
        *rows.last_mut().unwrap() = json!({"type":"response.incomplete","response":{"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"},"output":[]}});
    }
    rows.iter()
        .map(|row| format!("data: {row}\n\n"))
        .collect::<String>()
        .into_bytes()
}

#[tokio::test]
async fn model_contract_matrix_native_tools_use_real_wire_and_reject_broken_arguments() {
    for preset in LLM_PROVIDER_CAPABILITIES {
        for scenario in [
            "valid",
            "malformed-arguments",
            "duplicate-keys",
            "missing-name",
            "missing-finish",
            "truncated",
        ] {
            let protocol = preset.protocol;
            let server = LocalProvider::start(move |_| {
                ProviderResponse::bytes("text/event-stream", tool_events(protocol, scenario))
            })
            .await;
            let mut request = ChatRequest::system_user("工具查询", "查询雨夜", None, Some(256));
            request.tools = Some(vec![
                json!({"type":"function","function":{"name":"search","description":"检索","parameters":tool_schema()}}),
            ]);
            request.tool_choice = Some("required".into());
            let result =
                collect_chat_stream(provider(preset.id, &server.base_url, 2).as_ref(), request)
                    .await;
            let captured = server.requests();
            assert_eq!(captured.len(), 1);
            let body: Value = serde_json::from_slice(&captured[0].body).unwrap();
            let actual_schema = match protocol {
                LlmProtocolKind::OpenAiChatCompletions => {
                    &body["tools"][0]["function"]["parameters"]
                }
                LlmProtocolKind::OpenAiResponses => &body["tools"][0]["parameters"],
                LlmProtocolKind::AnthropicMessages => &body["tools"][0]["input_schema"],
                LlmProtocolKind::GeminiGenerateContent => {
                    &body["tools"][0]["functionDeclarations"][0]["parametersJsonSchema"]
                }
            };
            assert_eq!(actual_schema, &tool_schema(), "{} tool request", preset.id);
            if scenario == "valid" {
                let response = result
                    .as_ref()
                    .unwrap_or_else(|error| panic!("{}: {error}", preset.id));
                let calls = response.message.tool_calls.as_ref().unwrap();
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].name, "search");
                assert_eq!(calls[0].arguments, json!({"query":"雨夜"}));
                assert_eq!(response.finish_reason.as_deref(), Some("tool_calls"));
            }
            assert_case(
                "llm.transport",
                preset.id,
                "tools",
                scenario,
                json!(scenario == "valid"),
                json!(result.is_ok()),
                json!({"requests":captured.len(),"uri":captured[0].uri,"error":result.err().map(|error|error.to_string())}),
            );
        }
    }
}
