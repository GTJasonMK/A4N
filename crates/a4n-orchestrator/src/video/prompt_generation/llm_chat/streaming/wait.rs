use std::pin::Pin;
use std::time::{Duration, Instant};

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProvider};
use futures::{Stream, StreamExt};
use tokio::time::{Instant as TokioInstant, MissedTickBehavior, interval_at};
use tokio_util::sync::CancellationToken;

use super::super::super::super::VideoPromptProgress;
use super::super::contracts::{VideoPromptStepProgress, report_video_prompt_waiting};

const VIDEO_PROMPT_STREAM_HEARTBEAT: Duration = Duration::from_secs(8);

pub(super) async fn open_video_prompt_stream(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    cancel_token: Option<&CancellationToken>,
    step: VideoPromptStepProgress<'_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    started: Instant,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
    let mut request_future = Box::pin(llm.chat_stream(request));
    let mut heartbeat = interval_at(
        TokioInstant::now() + VIDEO_PROMPT_STREAM_HEARTBEAT,
        VIDEO_PROMPT_STREAM_HEARTBEAT,
    );
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        match cancel_token {
            Some(token) => {
                tokio::select! {
                    _ = token.cancelled() => return Err(AppError::Cancelled),
                    result = &mut request_future => return result,
                    _ = heartbeat.tick() => {
                        report_video_prompt_waiting(progress, step, step.current, started.elapsed(), "正在连接模型流")?;
                        tracing::debug!(
                            target: "a4n::video_prompt",
                            step = step.message,
                            duration_ms = started.elapsed().as_millis(),
                            "等待视频提示词 LLM stream 连接"
                        );
                    }
                }
            }
            None => {
                tokio::select! {
                    result = &mut request_future => return result,
                    _ = heartbeat.tick() => {
                        report_video_prompt_waiting(progress, step, step.current, started.elapsed(), "正在连接模型流")?;
                        tracing::debug!(
                            target: "a4n::video_prompt",
                            step = step.message,
                            duration_ms = started.elapsed().as_millis(),
                            "等待视频提示词 LLM stream 连接"
                        );
                    }
                }
            }
        }
    }
}

pub(super) async fn next_video_prompt_stream_item(
    stream: &mut Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
    cancel_token: Option<&CancellationToken>,
    step: VideoPromptStepProgress<'_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    started: Instant,
    phase: &str,
    current: u32,
) -> Result<Option<Result<ChatChunk, AppError>>, AppError> {
    let mut heartbeat = interval_at(
        TokioInstant::now() + VIDEO_PROMPT_STREAM_HEARTBEAT,
        VIDEO_PROMPT_STREAM_HEARTBEAT,
    );
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        match cancel_token {
            Some(token) => {
                tokio::select! {
                    _ = token.cancelled() => return Err(AppError::Cancelled),
                    item = stream.next() => return Ok(item),
                    _ = heartbeat.tick() => {
                        report_video_prompt_waiting(progress, step, current, started.elapsed(), phase)?;
                        tracing::debug!(
                            target: "a4n::video_prompt",
                            step = step.message,
                            duration_ms = started.elapsed().as_millis(),
                            "等待视频提示词 LLM stream chunk"
                        );
                    }
                }
            }
            None => {
                tokio::select! {
                    item = stream.next() => return Ok(item),
                    _ = heartbeat.tick() => {
                        report_video_prompt_waiting(progress, step, current, started.elapsed(), phase)?;
                        tracing::debug!(
                            target: "a4n::video_prompt",
                            step = step.message,
                            duration_ms = started.elapsed().as_millis(),
                            "等待视频提示词 LLM stream chunk"
                        );
                    }
                }
            }
        }
    }
}
