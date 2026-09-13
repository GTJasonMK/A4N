use std::time::Instant;

use a4n_llm::provider::{
    ChatMessage, ChatRequest, ChatResponse, LlmProtocolKind, MessageRole, collect_chat_chunks,
};
use futures::{executor::block_on, stream};
use serde_json::{Value, json};

use super::{chunk_stream, gemini, responses};

fn collect(
    protocol: LlmProtocolKind,
    events: &[Value],
) -> Result<ChatResponse, a4n_error::AppError> {
    let wire = events
        .iter()
        .map(|event| format!("event: message\r\ndata: {event}\r\n\r\n"))
        .collect::<String>();
    // Split inside UTF-8 characters and SSE delimiters, as a network stream can.
    let bytes: Vec<Result<Vec<u8>, reqwest::Error>> = wire
        .as_bytes()
        .chunks(7)
        .map(|bytes| Ok(bytes.to_vec()))
        .collect();
    let response = block_on(collect_chat_chunks(chunk_stream(
        stream::iter(bytes),
        protocol,
        1,
        "test".into(),
        Instant::now(),
    )))?;
    // Exercise the same serialization boundary used by sidecars and durable
    // Agent messages before replaying provider-specific tool context.
    Ok(serde_json::from_slice(&serde_json::to_vec(&response)?)?)
}

#[test]
fn responses_preserves_text_usage_and_reasoning_for_tool_replay() {
    let reasoning =
        json!({"type": "reasoning", "id": "rs_1", "encrypted_content": "opaque", "summary": []});
    let result = collect(LlmProtocolKind::OpenAiResponses, &[
        json!({"type": "response.output_text.delta", "delta": "查询上海"}),
        json!({"type": "response.reasoning_summary_text.delta", "delta": "先查天气"}),
        json!({"type": "response.completed", "response": {
            "status": "completed", "output": [reasoning.clone(), {
                "type": "function_call", "call_id": "call_1", "name": "weather", "arguments": "{\"city\":\"上海\"}"
            }], "usage": {"input_tokens": 5, "output_tokens": 3, "total_tokens": 8, "input_tokens_details": {"cached_tokens": 2}}
        }}),
    ]).unwrap();
    assert_eq!(result.message.content, "查询上海");
    assert_eq!(result.reasoning_content.as_deref(), Some("先查天气"));
    assert_eq!(result.finish_reason.as_deref(), Some("tool_calls"));
    assert_eq!(result.usage.unwrap().prompt_cache_hit_tokens, Some(2));
    let mut observation = ChatMessage::new(MessageRole::Tool, "晴天");
    observation.tool_call_id = Some("call_1".into());
    let mut request = ChatRequest::new(vec![result.message, observation], None, Some(100));
    request.tools = Some(vec![
        json!({"type": "function", "function": {"name": "weather", "parameters": {"type": "object"}}}),
    ]);
    let body = responses::request_body(&request, "model", Default::default()).unwrap();
    assert_eq!(body["input"][0], reasoning);
    assert_eq!(body["input"][2]["call_id"], "call_1");
    assert_eq!(body["input"][3]["type"], "function_call_output");
    assert_eq!(body["max_output_tokens"], 100);
    assert_eq!(body["store"], false);
    assert_eq!(body["tools"][0]["strict"], false);
    assert!(body["tools"][0].get("function").is_none());
}

#[test]
fn gemini_preserves_signature_parallel_calls_and_usage_after_finish() {
    let result = collect(LlmProtocolKind::GeminiGenerateContent, &[
        json!({"candidates": [{"content": {"parts": [{"text": "先查天气", "thought": true}]}}]}),
        json!({"candidates": [{"content": {"parts": [
            {"functionCall": {"name": "weather", "args": {"city": "上海"}}, "thoughtSignature": "opaque-signature"},
            {"functionCall": {"name": "weather", "args": {"city": "北京"}}}
        ]}, "finishReason": "STOP"}]}),
        json!({"usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 2, "thoughtsTokenCount": 1, "totalTokenCount": 8}}),
    ]).unwrap();
    let calls = result.message.tool_calls.as_ref().unwrap();
    assert_eq!(calls.len(), 2);
    assert_ne!(calls[0].id, calls[1].id);
    assert_eq!(result.finish_reason.as_deref(), Some("tool_calls"));
    assert_eq!(result.usage.unwrap().completion_tokens, 3);
    let mut observation = ChatMessage::new(MessageRole::Tool, "晴天");
    observation.tool_call_id = Some(calls[0].id.clone());
    let request = ChatRequest::new(vec![result.message, observation], None, Some(100));
    let body = gemini::request_body(&request, Default::default()).unwrap();
    assert_eq!(
        body["contents"][0]["parts"][0]["thoughtSignature"],
        "opaque-signature"
    );
    assert_eq!(
        body["contents"][1]["parts"][0]["functionResponse"]["name"],
        "weather"
    );
    assert_eq!(body["generationConfig"]["maxOutputTokens"], 100);
    let second = collect(LlmProtocolKind::GeminiGenerateContent, &[
        json!({"candidates": [{"content": {"parts": [{"functionCall": {"name": "weather", "args": {}}}]}, "finishReason": "STOP"}]}),
    ]).unwrap();
    assert_ne!(
        second.message.tool_calls.unwrap()[0].id,
        request.messages[0].tool_calls.as_ref().unwrap()[0].id
    );
}

#[test]
fn native_protocols_reject_missing_terminal_and_surface_truncation_or_errors() {
    assert!(
        collect(
            LlmProtocolKind::OpenAiResponses,
            &[json!({"type": "response.output_text.delta", "delta": "partial"})]
        )
        .is_err()
    );
    assert!(
        collect(
            LlmProtocolKind::GeminiGenerateContent,
            &[json!({"candidates": [{"content": {"parts": [{"text": "partial"}]}}]})]
        )
        .is_err()
    );
    assert!(
        collect(
            LlmProtocolKind::GeminiGenerateContent,
            &[json!({"promptFeedback": {"blockReason": "SAFETY"}})]
        )
        .is_err()
    );
    assert!(
        collect(
            LlmProtocolKind::OpenAiResponses,
            &[json!({"type": "response.failed", "response": {"error": {"message": "rate limit"}}})]
        )
        .is_err()
    );
    assert!(
        collect(LlmProtocolKind::OpenAiResponses, &[json!({"type": "response.incomplete", "response": {
            "status": "incomplete", "incomplete_details": {"reason": "max_output_tokens"}, "output": []
        }})])
        .is_err()
    );
}

#[test]
fn native_json_requests_map_owned_fields_and_reject_unsupported_options() {
    let mut request = ChatRequest::system_user("JSON only", "hello", Some(0.2), Some(100))
        .with_json_response_format();
    let responses = responses::request_body(&request, "model", Default::default()).unwrap();
    assert_eq!(responses["text"]["format"]["type"], "json_object");
    let gemini = gemini::request_body(&request, Default::default()).unwrap();
    assert_eq!(gemini["systemInstruction"]["parts"][0]["text"], "JSON only");
    assert_eq!(
        gemini["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert!(
        gemini::request_body(
            &request,
            serde_json::from_value(json!({"generationConfig": {"maxOutputTokens": 1}})).unwrap()
        )
        .is_err()
    );
    request.stop_sequences = Some(vec!["STOP".into()]);
    assert!(responses::request_body(&request, "model", Default::default()).is_err());
}

#[test]
fn responses_preserves_all_stable_boundaries_and_skips_dynamic_cache_writes() {
    let mut request =
        ChatRequest::system_user("stable rules", "shared context\nnew task", None, None);
    request
        .add_cache_breakpoint(1, "shared context".len())
        .unwrap();
    request
        .configure_prompt_cache("openai-responses", "gpt-5.6-luna", None, Some(7))
        .unwrap();
    let body = responses::request_body(&request, "gpt-5.6-luna", Default::default()).unwrap();
    assert_eq!(body["prompt_cache_options"]["mode"], "explicit");
    for (index, text) in [(0, "stable rules"), (1, "shared context")] {
        assert_eq!(body["input"][index]["content"][0]["text"], text);
        assert_eq!(
            body["input"][index]["content"][0]["prompt_cache_breakpoint"]["mode"],
            "explicit"
        );
    }
    assert_eq!(body["input"][1]["content"][1]["text"], "\nnew task");
    assert!(
        body["input"][1]["content"][1]
            .get("prompt_cache_breakpoint")
            .is_none()
    );
}

#[test]
fn responses_caches_native_dialogue_from_its_first_user_message() {
    let mut request = ChatRequest::system_user("agent rules", "initial task", None, None);
    request
        .configure_prompt_cache("openai-responses", "gpt-5.6-luna", None, Some(7))
        .unwrap();
    let body = responses::request_body(&request, "gpt-5.6-luna", Default::default()).unwrap();
    assert_eq!(body["prompt_cache_options"]["mode"], "implicit");
    assert_eq!(body["input"][1]["content"], "initial task");
}

#[test]
fn native_usage_normalizes_cache_misses_and_preserves_write_tokens() {
    let response = collect(
        LlmProtocolKind::OpenAiResponses,
        &[json!({"type": "response.completed", "response": {
            "status": "completed", "output": [],
            "usage": {"input_tokens": 10000, "output_tokens": 100, "total_tokens": 10100,
                "input_tokens_details": {"cached_tokens": 9000, "cache_write_tokens": 400}}
        }})],
    )
    .unwrap();
    let usage = response.usage.unwrap();
    assert_eq!(usage.prompt_cache_hit_tokens, Some(9000));
    assert_eq!(usage.prompt_cache_miss_tokens, Some(1000));
    assert_eq!(usage.prompt_cache_write_tokens, Some(400));
    let response = collect(LlmProtocolKind::GeminiGenerateContent, &[
        json!({"candidates": [{"content": {"parts": [{"text": "ok"}]}, "finishReason": "STOP"}]}),
        json!({"usageMetadata": {"promptTokenCount": 10000, "totalTokenCount": 10100,
            "cachedContentTokenCount": 9000}})
    ]).unwrap();
    assert_eq!(response.usage.unwrap().prompt_cache_miss_tokens, Some(1000));
    let error = collect(LlmProtocolKind::GeminiGenerateContent, &[
        json!({"candidates": [{"content": {"parts": [{"text": "ok"}]}, "finishReason": "STOP"}],
            "usageMetadata": {"promptTokenCount": 100, "totalTokenCount": 101, "cachedContentTokenCount": 101}})
    ]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("缓存命中 token 数超过 prompt token 数")
    );
}
