use std::pin::Pin;
use std::time::Instant;

use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest};
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
    let response = provider
        .build_request(&request, true)
        .send()
        .await
        .map_err(|error| {
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

    let stream = chat_chunk_stream(
        response.bytes_stream(),
        provider.timeout_secs,
        provider.model.clone(),
        request_started,
    );

    Ok(Box::pin(stream))
}
