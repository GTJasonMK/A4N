use std::{future::Future, pin::Pin, time::Duration};

use a4n_error::AppError;
use futures::{Stream, StreamExt, stream};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use super::{ChatChunk, classify_finish_reason};

pub type ChatChunkStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>;
pub const LLM_CALL_TOTAL_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Covers preparation, admission and consumption. Network connect/read limits
/// remain in the transport, so waiting for a slot is never a connect timeout.
pub async fn open_chat_stream(
    opening: impl Future<Output = Result<ChatChunkStream, AppError>>,
    cancellation: CancellationToken,
) -> Result<ChatChunkStream, AppError> {
    open_until(
        opening,
        cancellation,
        Instant::now() + LLM_CALL_TOTAL_TIMEOUT,
    )
    .await
}

async fn open_until(
    opening: impl Future<Output = Result<ChatChunkStream, AppError>>,
    cancellation: CancellationToken,
    deadline: Instant,
) -> Result<ChatChunkStream, AppError> {
    let inner = tokio::select! {
        biased;
        _ = cancellation.cancelled() => return Err(AppError::Cancelled),
        _ = tokio::time::sleep_until(deadline) => return Err(total_timeout()),
        result = opening => result?,
    };
    Ok(Box::pin(stream::unfold(
        Some((inner, cancellation, deadline, None::<String>)),
        |state| async move {
            let (mut inner, cancellation, deadline, mut finish_reason) = state?;
            let item = tokio::select! {
                biased;
                _ = cancellation.cancelled() => return Some((Err(AppError::Cancelled), None)),
                _ = tokio::time::sleep_until(deadline) => return Some((Err(total_timeout()), None)),
                item = inner.next() => item,
            };
            match item {
                Some(Ok(chunk)) => {
                    if chunk.finish_reason.is_some() {
                        finish_reason = chunk.finish_reason.clone();
                    }
                    Some((
                        Ok(chunk),
                        Some((inner, cancellation, deadline, finish_reason)),
                    ))
                }
                Some(Err(error)) => Some((Err(error), None)),
                None => match classify_finish_reason(finish_reason.as_deref()) {
                    Ok(_) => None,
                    Err(error) => Some((Err(error), None)),
                },
            }
        },
    )))
}

fn total_timeout() -> AppError {
    AppError::Timeout("LLM 调用超过总时限（包含准备、排队与模型输出）".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancellation_interrupts_pending_open_and_pending_output() {
        let cancel = CancellationToken::new();
        let opening_started = std::sync::Arc::new(tokio::sync::Notify::new());
        let started = opening_started.clone();
        let token = cancel.clone();
        let canceller = tokio::spawn(async move {
            started.notified().await;
            token.cancel();
        });
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            open_chat_stream(
                async {
                    opening_started.notify_one();
                    std::future::pending().await
                },
                cancel,
            ),
        )
        .await
        .unwrap();
        assert!(matches!(result, Err(AppError::Cancelled)));
        canceller.await.unwrap();

        let cancel = CancellationToken::new();
        let mut stream = open_chat_stream(
            async { Ok(Box::pin(stream::pending()) as ChatChunkStream) },
            cancel.clone(),
        )
        .await
        .unwrap();
        cancel.cancel();
        assert!(matches!(
            stream.next().await,
            Some(Err(AppError::Cancelled))
        ));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn deadline_covers_opening_and_stream_without_resetting_on_chunks() {
        let result = open_until(
            std::future::pending(),
            CancellationToken::new(),
            Instant::now(),
        )
        .await;
        assert!(matches!(result, Err(AppError::Timeout(_))));
        let deadline = Instant::now() + Duration::from_millis(10);
        let mut stream = open_until(
            async { Ok(Box::pin(stream::pending()) as ChatChunkStream) },
            CancellationToken::new(),
            deadline,
        )
        .await
        .unwrap();
        tokio::time::sleep_until(deadline).await;
        assert!(matches!(
            stream.next().await,
            Some(Err(AppError::Timeout(_)))
        ));
        assert!(stream.next().await.is_none());
    }
}
