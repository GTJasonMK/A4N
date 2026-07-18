use super::errors::stream_read_error;
use super::sse::{SseDrainOutcome, drain_sse_lines, drain_sse_tail};
use a4n_core::error::AppError;
use a4n_llm::provider::ChatChunk;
use futures::{Stream, StreamExt};
use std::collections::VecDeque;
use std::pin::Pin;
use std::time::Instant;

struct ChatStreamState<S> {
    byte_stream: Pin<Box<S>>,
    buffer: Vec<u8>,
    queue: VecDeque<Result<ChatChunk, AppError>>,
    finished: bool,
    terminal_chunk_seen: bool,
    received_bytes: usize,
    parsed_chunks: usize,
}

impl<S> ChatStreamState<S> {
    fn observe(&mut self, outcome: SseDrainOutcome) {
        self.finished |= outcome.done;
        self.terminal_chunk_seen |= outcome.terminal_chunk_seen;
        self.parsed_chunks = self.parsed_chunks.saturating_add(outcome.parsed_chunks);
    }
}

pub(super) fn chat_chunk_stream<S, B>(
    byte_stream: S,
    read_timeout_secs: u64,
    model: String,
    request_started: Instant,
) -> impl Stream<Item = Result<ChatChunk, AppError>> + Send
where
    S: Stream<Item = Result<B, reqwest::Error>> + Send + 'static,
    B: AsRef<[u8]> + Send + 'static,
{
    futures::stream::unfold(
        ChatStreamState {
            byte_stream: Box::pin(byte_stream),
            buffer: Vec::new(),
            queue: VecDeque::new(),
            finished: false,
            terminal_chunk_seen: false,
            received_bytes: 0,
            parsed_chunks: 0,
        },
        move |mut state| {
            let model = model.clone();
            async move {
                loop {
                    if let Some(item) = state.queue.pop_front() {
                        return Some((item, state));
                    }
                    if state.finished {
                        return None;
                    }

                    match state.byte_stream.as_mut().next().await {
                        Some(Ok(bytes)) => {
                            let bytes = bytes.as_ref();
                            state.received_bytes = state.received_bytes.saturating_add(bytes.len());
                            state.buffer.extend_from_slice(bytes);
                            let outcome = drain_sse_lines(&mut state.buffer, &mut state.queue);
                            state.observe(outcome);
                        }
                        Some(Err(error)) => {
                            state.finished = true;
                            if state.terminal_chunk_seen {
                                // finish_reason 已声明模型终态；其后的 usage/[DONE] 属于可选尾帧，
                                // transport 尾错不能推翻已完整收到的内容和截断原因。
                                continue;
                            }
                            let error = stream_read_error(
                                &error,
                                read_timeout_secs,
                                &model,
                                state.received_bytes,
                                state.parsed_chunks,
                                request_started.elapsed(),
                            );
                            return Some((Err(error), state));
                        }
                        None => {
                            let outcome = drain_sse_tail(&mut state.buffer, &mut state.queue);
                            state.observe(outcome);
                            state.finished = true;
                        }
                    }
                }
            }
        },
    )
}
