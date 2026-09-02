use super::errors::*;
use super::provider::HttpLlmProvider;
use super::response::*;
use super::sse::*;
use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, ChatMessage, ChatRequest, normalize_extra_body};
use reqwest::StatusCode;
use serde_json::{Value, json};

fn parse_data_line(line: &str) -> Result<ChatChunk, AppError> {
    match parse_sse_data_line(line.as_bytes()) {
        ParsedSseLine::Chunk(item) => item,
        ParsedSseLine::Ignored => panic!("data line should not be ignored"),
        ParsedSseLine::Done => panic!("data line should contain a chunk"),
    }
}

fn request_body_json(request: reqwest::RequestBuilder) -> Value {
    let request = request.build().expect("request should build");
    let body = request
        .body()
        .and_then(reqwest::Body::as_bytes)
        .expect("request should have an in-memory JSON body");
    serde_json::from_slice(body).expect("request body should be JSON")
}

#[test]
fn parse_sse_data_line_accepts_valid_chunk() {
    let chunk = parse_data_line(
        r#"data: {"choices":[{"index":2,"delta":{"content":"你好"},"finish_reason":"stop"}]}"#,
    )
    .expect("valid chunk");

    assert_eq!(chunk.content.as_deref(), Some("你好"));
    assert_eq!(chunk.reasoning_content.as_deref(), None);
    assert_eq!(chunk.finish_reason.as_deref(), Some("stop"));
    assert_eq!(chunk.index, 2);
    assert!(chunk.usage.is_none());
}

#[test]
fn parse_sse_data_line_accepts_reasoning_chunk_without_content() {
    let chunk = parse_data_line(
        r#"data: {"choices":[{"index":0,"delta":{"content":null,"reasoning_content":"正在分析"},"finish_reason":null}]}"#,
    )
    .expect("reasoning chunk");

    assert_eq!(chunk.content.as_deref(), None);
    assert_eq!(chunk.reasoning_content.as_deref(), Some("正在分析"));
    assert_eq!(chunk.finish_reason.as_deref(), None);
    assert_eq!(chunk.index, 0);
    assert!(chunk.usage.is_none());
}

#[test]
fn parse_sse_data_line_keeps_openai_tool_call_deltas() {
    let first_line = format!(
        "data: {}",
        json!({
            "choices": [{
                "index": 0,
                "delta": {"tool_calls": [{
                    "index": 0,
                    "id": "call-1",
                    "function": {"name": "locate", "arguments": "{\"project_id\":"}
                }]},
                "finish_reason": null
            }]
        })
    );
    let first = parse_data_line(&first_line).expect("tool-call delta");
    let call = first
        .tool_calls
        .expect("tool call should be parsed")
        .remove(0);
    assert_eq!(call.id, "call-1");
    assert_eq!(call.name, "locate");
    assert_eq!(call.arguments, Value::String("{\"project_id\":".into()));
    assert_eq!(call.stream_index, Some(0));

    let continuation = parse_data_line(
        r#"data: {"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"7}"}}]},"finish_reason":"tool_calls"}]}"#,
    )
    .expect("tool-call continuation");
    let call = continuation
        .tool_calls
        .expect("continuation should be parsed")
        .remove(0);
    assert_eq!(call.id, "__stream_tool_0");
    assert_eq!(call.stream_index, Some(0));
    assert_eq!(call.arguments, Value::String("7}".into()));
}

#[test]
fn parse_sse_data_line_recognizes_done_marker() {
    assert!(matches!(
        parse_sse_data_line(b"data: [DONE]"),
        ParsedSseLine::Done
    ));
}

#[test]
fn drain_sse_lines_stops_at_done_and_records_terminal_chunk() {
    let mut buffer = concat!(
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
        "data: {not valid json}\n\n"
    )
    .as_bytes()
    .to_vec();
    let mut queue = std::collections::VecDeque::new();

    let outcome = drain_sse_lines(&mut buffer, &mut queue);

    assert!(outcome.done);
    assert!(outcome.terminal_chunk_seen);
    assert_eq!(outcome.parsed_chunks, 1);
    assert!(buffer.is_empty());
    assert_eq!(queue.len(), 1);
    assert!(queue.pop_front().expect("terminal chunk").is_ok());
}

#[test]
fn parse_sse_data_line_rejects_empty_choices() {
    let err =
        parse_data_line(r#"data: {"choices":[]}"#).expect_err("empty choices should be rejected");

    let message = err.to_string();
    assert!(message.contains("choices"));
    assert!(message.contains("响应片段"));
}

#[test]
fn parse_sse_data_line_emits_usage_only_empty_choices_chunk() {
    let chunk = parse_data_line(
        r#"data: {"id":"chatcmpl-test","choices":[],"usage":{"prompt_tokens":12,"completion_tokens":34,"total_tokens":46,"prompt_cache_hit_tokens":10,"prompt_cache_miss_tokens":2}}"#,
    )
    .expect("usage-only chunk should be emitted");

    assert!(chunk.content.is_none());
    assert!(chunk.reasoning_content.is_none());
    let usage = chunk.usage.expect("usage-only chunk should carry usage");
    assert_eq!(usage.prompt_tokens, 12);
    assert_eq!(usage.completion_tokens, 34);
    assert_eq!(usage.total_tokens, 46);
    assert_eq!(usage.prompt_cache_hit_tokens, Some(10));
    assert_eq!(usage.prompt_cache_miss_tokens, Some(2));
}

#[test]
fn parse_sse_data_line_keeps_usage_on_choice_chunk() {
    let chunk = parse_data_line(
        r#"data: {"id":"chatcmpl-test","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":34,"total_tokens":46,"prompt_cache_hit_tokens":10,"prompt_cache_miss_tokens":2}}"#,
    )
    .expect("choice chunk with usage should parse");

    assert_eq!(chunk.finish_reason.as_deref(), Some("stop"));
    let usage = chunk.usage.expect("choice chunk should carry usage");
    assert_eq!(usage.prompt_tokens, 12);
    assert_eq!(usage.completion_tokens, 34);
    assert_eq!(usage.total_tokens, 46);
    assert_eq!(usage.prompt_cache_hit_tokens, Some(10));
    assert_eq!(usage.prompt_cache_miss_tokens, Some(2));
}

#[test]
fn parse_sse_data_line_rejects_empty_choices_with_upstream_error() {
    let err = parse_data_line(
        r#"data: {"choices":[],"error":{"message":"context length exceeded","code":"context_length_exceeded"}}"#,
    )
    .expect_err("empty choices with error should be rejected");

    let message = err.to_string();
    assert!(message.contains("context length exceeded"));
    assert!(message.contains("code=context_length_exceeded"));
}

#[test]
fn first_choice_reports_upstream_error_for_empty_choices() {
    let body = r#"{"id":"chatcmpl-test","choices":[],"error":{"message":"context length exceeded","type":"invalid_request_error","code":"context_length_exceeded"}}"#;
    let json = parse_response_json(body, "LLM API").expect("valid response json");
    let err = first_choice(&json, "choices", Some(body)).expect_err("empty choices should fail");

    let message = err.to_string();
    assert!(message.contains("choices"));
    assert!(message.contains("context length exceeded"));
    assert!(message.contains("type=invalid_request_error"));
    assert!(message.contains("code=context_length_exceeded"));
    assert!(message.contains("chatcmpl-test"));
}

#[test]
fn first_choice_reports_top_level_message_for_empty_choices() {
    let body = r#"{"choices":[],"message":"content filtered by upstream service","code":"content_filter"}"#;
    let json = parse_response_json(body, "LLM API").expect("valid response json");
    let err = first_choice(&json, "choices", Some(body)).expect_err("empty choices should fail");

    let message = err.to_string();
    assert!(message.contains("content filtered by upstream service"));
    assert!(message.contains("code=content_filter"));
    assert!(message.contains("响应片段"));
}

#[test]
fn payment_required_status_is_configuration_error() {
    let body = r#"{"error":{"message":"Insufficient Balance","type":"unknown_error","code":"invalid_request_error"}}"#;
    let err = llm_api_status_error(StatusCode::PAYMENT_REQUIRED, body);

    let message = err.to_string();
    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(message.contains("余额不足"));
    assert!(message.contains("Insufficient Balance"));
    assert!(!message.contains("内部错误"));
}

#[test]
fn cloudflare_524_explains_first_byte_timeout_and_streaming_fix() {
    let status = StatusCode::from_u16(524).expect("524 is a valid extension status");
    let err = llm_api_status_error(status, "error code: 524");

    let message = err.to_string();
    assert!(matches!(err, AppError::LlmError(_)));
    assert!(message.contains("上游代理等待模型响应超时"));
    assert!(message.contains("流式调用"));
}

#[test]
fn extra_body_reject_reserved_keys() {
    let err = normalize_extra_body(Some(json!({
        "model": "override"
    })))
    .expect_err("reserved request option key should be rejected");

    assert!(err.to_string().contains("系统保留字段"));
}

#[test]
fn extra_body_reject_response_format_as_request_contract() {
    let err = normalize_extra_body(Some(json!({
        "response_format": { "type": "json_object" }
    })))
    .expect_err("response_format is controlled per request");

    assert!(err.to_string().contains("系统保留字段"));
}

#[test]
fn extra_body_accept_provider_extension_fields() {
    let options = normalize_extra_body(Some(json!({
        "reasoning_effort": "low"
    })))
    .expect("provider extension field should be accepted")
    .expect("non-empty options");

    assert_eq!(options["reasoning_effort"], "low");
}

#[test]
fn connect_timeout_is_derived_from_llm_timeout_with_bounds() {
    assert_eq!(normalized_connect_timeout_secs(1), MIN_CONNECT_TIMEOUT_SECS);
    assert_eq!(normalized_connect_timeout_secs(30), 30);
    assert_eq!(
        normalized_connect_timeout_secs(300),
        MAX_CONNECT_TIMEOUT_SECS
    );
}

#[test]
fn non_timeout_request_failure_is_an_llm_error() {
    let error = reqwest::Client::new()
        .get("not a valid URL")
        .build()
        .expect_err("invalid URL should fail");

    let error = request_error("LLM API", error, 30, 10, std::time::Duration::ZERO);

    assert!(matches!(error, AppError::LlmError(_)));
}

#[test]
fn provider_uses_derived_connect_timeout() {
    let provider =
        HttpLlmProvider::new_with_timeout("https://example.test", "deepseek-chat", None, 300);

    assert_eq!(provider.timeout_secs, 300);
    assert_eq!(provider.connect_timeout_secs, MAX_CONNECT_TIMEOUT_SECS);
}

#[test]
fn stream_request_includes_usage_only_when_requested() {
    let provider =
        HttpLlmProvider::new_with_timeout("https://example.test", "deepseek-chat", None, 120);
    let base_request = ChatRequest::system_user("system", "user", Some(0.2), None, None);

    let without_usage = request_body_json(provider.build_request(&base_request, true));
    assert_eq!(without_usage["stream"], json!(true));
    assert!(without_usage.get("stream_options").is_none());

    let with_usage =
        request_body_json(provider.build_request(&base_request.with_stream_usage(), true));
    assert_eq!(with_usage["stream"], json!(true));
    assert_eq!(with_usage["stream_options"]["include_usage"], json!(true));
}

#[test]
fn stream_request_usage_merges_existing_stream_options() {
    let provider = HttpLlmProvider::new_with_timeout_and_options(
        "https://example.test",
        "deepseek-chat",
        None,
        120,
        Some(json!({
            "stream_options": {
                "existing_provider_option": "kept"
            }
        })),
    )
    .expect("provider options should be accepted");
    let request =
        ChatRequest::system_user("system", "user", Some(0.2), None, None).with_stream_usage();

    let body = request_body_json(provider.build_request(&request, true));
    assert_eq!(body["stream_options"]["include_usage"], json!(true));
    assert_eq!(
        body["stream_options"]["existing_provider_option"],
        json!("kept")
    );
}

#[test]
fn request_marks_the_stable_system_prefix_for_gpt_5_6_cache_reuse() {
    let provider =
        HttpLlmProvider::new_with_timeout("https://example.test", "gpt-5.6-luna", None, 120);
    let mut request = ChatRequest::new(
        vec![
            ChatMessage::system("stable session"),
            ChatMessage::system("stable shared context"),
            ChatMessage::user("dynamic user"),
        ],
        None,
        None,
        None,
    );
    request.cache_scope_key = Some("a4n:workflow_run:299".to_string());
    request.cache_system_prefix = true;

    let body = request_body_json(provider.build_request(&request, true));
    assert_eq!(body["prompt_cache_key"], json!("a4n:workflow_run:299"));
    assert_eq!(body["messages"][0]["content"], "stable session");
    assert_eq!(
        body["messages"][1]["content"][0]["text"],
        "stable shared context"
    );
    assert_eq!(
        body["messages"][1]["content"][0]["prompt_cache_breakpoint"]["mode"],
        "explicit"
    );
    assert_eq!(body["messages"][2]["content"], "dynamic user");
    assert!(body.get("prompt_cache_options").is_none());
}

#[test]
fn request_marks_the_manifest_stable_prefix_before_dynamic_user_content() {
    let provider =
        HttpLlmProvider::new_with_timeout("https://example.test", "gpt-5.6-luna", None, 120);
    let mut request = ChatRequest::system_user(
        "stable session",
        "[稳定上下文]\nshared chapter\n\n[当前任务]\npage 2",
        None,
        None,
        None,
    );
    request.cache_scope_key = Some("a4n:workflow_run:299:prefix:abc".to_string());
    request.cache_system_prefix = true;
    request.cache_user_prefix_len = Some("[稳定上下文]\nshared chapter".len());

    let body = request_body_json(provider.build_request(&request, true));
    assert_eq!(
        body["messages"][1]["content"][0]["text"],
        "[稳定上下文]\nshared chapter"
    );
    assert_eq!(
        body["messages"][1]["content"][0]["prompt_cache_breakpoint"]["mode"],
        "explicit"
    );
    assert_eq!(
        body["messages"][1]["content"][1]["text"],
        "\n\n[当前任务]\npage 2"
    );
    assert_eq!(body["messages"][0]["content"], "stable session");
}

#[test]
fn request_includes_json_response_format_when_requested() {
    let provider =
        HttpLlmProvider::new_with_timeout("https://example.test", "deepseek-chat", None, 120);
    let request = ChatRequest::system_user("system", "user", Some(0.2), None, None)
        .with_json_response_format();

    let body = request_body_json(provider.build_request(&request, false));
    assert_eq!(body["response_format"]["type"], json!("json_object"));
}

#[test]
fn parse_sse_data_line_requires_choice_index() {
    let err = parse_data_line(r#"data: {"choices":[{"delta":{"content":"x"}}]}"#)
        .expect_err("missing index should be rejected");

    assert!(err.to_string().contains("choices[0].index"));
}

#[test]
fn parse_response_json_includes_body_preview_on_invalid_json() {
    let err = parse_response_json("<html>bad gateway</html>", "LLM API")
        .expect_err("invalid json should include response preview");

    let message = err.to_string();
    assert!(message.contains("解析LLM API响应 JSON 失败"));
    assert!(message.contains("<html>bad gateway</html>"));
}

#[test]
fn usage_info_accepts_complete_usage() {
    let usage = usage_info_from_json(&json!({
        "prompt_tokens": 3,
        "completion_tokens": 5,
        "total_tokens": 8
    }))
    .expect("complete usage");

    assert_eq!(usage.prompt_tokens, 3);
    assert_eq!(usage.completion_tokens, 5);
    assert_eq!(usage.total_tokens, 8);
    assert_eq!(usage.prompt_cache_hit_tokens, None);
    assert_eq!(usage.prompt_cache_miss_tokens, None);
}

#[test]
fn usage_info_accepts_deepseek_cache_fields() {
    let usage = usage_info_from_json(&json!({
        "prompt_tokens": 100,
        "completion_tokens": 20,
        "total_tokens": 120,
        "prompt_cache_hit_tokens": 80,
        "prompt_cache_miss_tokens": 20
    }))
    .expect("deepseek usage");

    assert_eq!(usage.prompt_tokens, 100);
    assert_eq!(usage.prompt_cache_hit_tokens, Some(80));
    assert_eq!(usage.prompt_cache_miss_tokens, Some(20));
}

#[test]
fn usage_info_accepts_openai_cached_tokens_detail() {
    let usage = usage_info_from_json(&json!({
        "prompt_tokens": 100,
        "completion_tokens": 20,
        "total_tokens": 120,
        "prompt_tokens_details": {
            "cached_tokens": 75
        }
    }))
    .expect("openai usage");

    assert_eq!(usage.prompt_cache_hit_tokens, Some(75));
    assert_eq!(usage.prompt_cache_miss_tokens, Some(25));
}

#[test]
fn usage_info_rejects_cache_hits_above_prompt_tokens() {
    let err = usage_info_from_json(&json!({
        "prompt_tokens": 100,
        "completion_tokens": 20,
        "total_tokens": 120,
        "prompt_tokens_details": {
            "cached_tokens": 101
        }
    }))
    .expect_err("cache hit count above prompt tokens must be visible");

    assert!(
        err.to_string()
            .contains("缓存命中 token 数超过 prompt token 数")
    );
}

#[test]
fn usage_info_requires_all_token_fields() {
    let err = usage_info_from_json(&json!({
        "prompt_tokens": 3,
        "total_tokens": 8
    }))
    .expect_err("missing completion token count should be rejected");

    assert!(err.to_string().contains("usage.completion_tokens"));
}
