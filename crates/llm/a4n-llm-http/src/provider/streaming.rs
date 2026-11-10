use std::pin::Pin;
use std::time::Instant;

use a4n_error::AppError;
use a4n_llm_provider_contract::{ChatChunk, ChatRequest, LlmProtocolKind};
use futures::Stream;

use crate::errors::{request_error, response_text};
use crate::response::llm_api_status_error_with_context;
use crate::stream::chat_chunk_stream;

use super::HttpLlmProvider;

pub(super) async fn execute_chat_stream(
    provider: &HttpLlmProvider,
    request: ChatRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
    let request_started = Instant::now();
    let builder = if provider.protocol == LlmProtocolKind::OpenAiChatCompletions {
        provider.build_request(&request, true)?
    } else {
        provider.build_native_request(&request)?
    };
    let response = crate::transport::send(builder).await.map_err(|error| {
        request_error(
            "LLM API 流式",
            error,
            provider.timeout_secs,
            provider.connect_timeout_secs,
            request_started.elapsed(),
        )
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let response_context = upstream_response_context(response.headers());
        let body = response_text(response, "LLM API", provider.timeout_secs).await?;
        return Err(llm_api_status_error_with_context(
            status,
            &body,
            &response_context,
        ));
    }

    if provider.protocol != LlmProtocolKind::OpenAiChatCompletions {
        return Ok(Box::pin(crate::native::chunk_stream(
            response.bytes_stream(),
            provider.protocol,
            provider.timeout_secs,
            provider.model.clone(),
            request_started,
        )));
    }
    let stream = chat_chunk_stream(
        response.bytes_stream(),
        provider.timeout_secs,
        provider.model.clone(),
        request_started,
    );

    Ok(Box::pin(stream))
}

fn upstream_response_context(headers: &reqwest::header::HeaderMap) -> String {
    ["x-request-id", "request-id", "x-correlation-id", "cf-ray"]
        .into_iter()
        .filter_map(|name| {
            let value = headers.get(name)?.to_str().ok()?;
            let value = value
                .chars()
                .filter(|character| character.is_ascii_alphanumeric() || "-_.".contains(*character))
                .take(128)
                .collect::<String>();
            (!value.is_empty()).then(|| format!("{name}={value}"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}
