use std::time::Instant;

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatMessage, ChatRequest, ChatResponse, MessageRole};

use crate::http::errors::{request_error, response_text};
use crate::http::response::{
    first_choice, llm_api_status_error, parse_response_json, required_object, required_string,
    usage_info_from_json,
};

use super::HttpLlmProvider;

pub(super) async fn execute_chat(
    provider: &HttpLlmProvider,
    request: ChatRequest,
) -> Result<ChatResponse, AppError> {
    let request_started = Instant::now();
    let response = provider
        .build_request(&request, false)
        .send()
        .await
        .map_err(|error| {
            request_error(
                "LLM API",
                error,
                provider.timeout_secs,
                provider.connect_timeout_secs,
                request_started.elapsed(),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response_text(response, "LLM API", provider.timeout_secs).await?;
        return Err(llm_api_status_error(status, &body));
    }

    let body = response_text(response, "LLM API", provider.timeout_secs).await?;
    let response_json = parse_response_json(&body, "LLM API")?;
    let choice = first_choice(&response_json, "choices", Some(&body))?;
    let message = &choice["message"];
    required_object(message, "choices[0].message")?;

    let content = required_string(&message["content"], "choices[0].message.content")?.to_string();
    let finish_reason = choice["finish_reason"].as_str().map(str::to_string);
    let usage = response_json
        .get("usage")
        .map(usage_info_from_json)
        .transpose()?;

    Ok(ChatResponse {
        message: ChatMessage {
            role: MessageRole::Assistant,
            content,
            tool_calls: None,
            tool_call_id: None,
        },
        usage,
        finish_reason,
    })
}
