use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, LlmProvider};
use tokio_util::sync::CancellationToken;

use super::chunks::collect_stream_chunks;

pub(super) async fn stream_chapter_response<F>(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    cancel_token: Option<&CancellationToken>,
    on_chunk: &mut F,
) -> Result<(String, Option<String>), AppError>
where
    F: FnMut(&str, u32),
{
    let stream_result = if let Some(cancel_token) = cancel_token {
        tokio::select! {
            _ = cancel_token.cancelled() => return Err(AppError::Cancelled),
            result = llm.chat_stream(request) => result,
        }
    } else {
        llm.chat_stream(request).await
    };
    let mut stream = stream_result?;
    collect_stream_chunks(&mut stream, cancel_token, on_chunk).await
}
