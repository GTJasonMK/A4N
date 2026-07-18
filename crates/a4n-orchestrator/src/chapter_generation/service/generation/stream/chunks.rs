use std::pin::Pin;

use a4n_core::error::AppError;
use a4n_llm::provider::ChatChunk;
use futures::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;

pub(super) async fn collect_stream_chunks<F>(
    stream: &mut Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
    cancel_token: Option<&CancellationToken>,
    on_chunk: &mut F,
) -> Result<(String, Option<String>), AppError>
where
    F: FnMut(&str, u32),
{
    let mut accumulated = String::new();
    let mut index = 0_u32;
    let mut finish_reason = None;
    loop {
        let item = if let Some(cancel_token) = cancel_token {
            tokio::select! {
                _ = cancel_token.cancelled() => return Err(AppError::Cancelled),
                item = stream.next() => item,
            }
        } else {
            stream.next().await
        };
        let Some(item) = item else { break };
        let chunk = item?;
        if let Some(delta) = chunk.content.as_deref()
            && !delta.is_empty()
        {
            accumulated.push_str(delta);
            on_chunk(delta, index);
            index += 1;
        }
        if chunk.finish_reason.is_some() {
            finish_reason = chunk.finish_reason;
            break;
        }
    }
    Ok((accumulated, finish_reason))
}
