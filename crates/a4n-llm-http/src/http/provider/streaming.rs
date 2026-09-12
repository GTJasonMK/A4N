use std::pin::Pin;
use std::time::Instant;

use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProtocolKind};
use futures::Stream;

use crate::http::errors::{request_error, response_text};
use crate::http::response::llm_api_status_error;
use crate::http::stream::chat_chunk_stream;

use super::HttpLlmProvider;

pub(super) async fn execute_chat_stream(
    provider: &HttpLlmProvider,
    request: ChatRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
    let request_started = Instant::now();
    let builder = if provider.protocol == LlmProtocolKind::OpenAiChatCompletions {
        provider.build_request(&request, true)
    } else {
        provider.build_native_request(&request)?
    };
    let response = builder.send().await.map_err(|error| {
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
        let body = response_text(response, "LLM API", provider.timeout_secs).await?;
        return Err(llm_api_status_error(status, &body));
    }

    if provider.protocol != LlmProtocolKind::OpenAiChatCompletions {
        return Ok(Box::pin(crate::http::native::chunk_stream(
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
