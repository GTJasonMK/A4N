//! 大纲 generation 任务内的统一 LLM stream 上下文。
//!
//! 生产大纲 LLM 调用必须走 `execute_llm_generation`，禁止 `chat_llm` 完整响应。

use std::future::Future;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatMessage, ChatRequest, ChatResponse};
use tokio::task_local;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;
use crate::generation::{
    checkpoint::{GenerationCheckpointMetadata, GenerationCheckpointWriter},
    events::{GenerationTaskSession, SessionGenerationEventPublisher},
    generation_finish_reason_text,
    stream::{
        LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation, provider_usage,
    },
};
use a4n_service_protocol::GenerationError;

task_local! {
    static OUTLINE_LLM_STREAM: OutlineLlmStreamContext;
}

#[derive(Clone)]
pub(in crate::outline) struct OutlineLlmStreamContext {
    pub publisher: SessionGenerationEventPublisher,
    pub session: Arc<GenerationTaskSession>,
    pub checkpoint_writer: GenerationCheckpointWriter,
    pub cancellation: CancellationToken,
}

pub(in crate::outline) async fn with_outline_llm_stream<F, Fut, T>(
    context: OutlineLlmStreamContext,
    work: F,
) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    OUTLINE_LLM_STREAM.scope(context, work()).await
}

/// 在大纲 generation 任务上下文中执行统一 stream driver。
pub(in crate::outline) async fn execute_outline_llm_call(
    context: &OrchestratorContext,
    request: ChatRequest,
    call_key: &str,
) -> Result<ChatResponse, AppError> {
    let stream = OUTLINE_LLM_STREAM
        .try_with(|value| value.clone())
        .map_err(|_| AppError::InvalidState {
            current: "outline_llm_stream_missing".to_string(),
            operation: format!("大纲 LLM 调用 {call_key} 必须在 generation stream 上下文中执行"),
        })?;
    if stream.cancellation.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    let output = execute_llm_generation(
        context.llm_backend(),
        &stream.publisher,
        LlmGenerationRequest {
            request,
            stream_id: "raw_json".to_string(),
            cancellation: stream.cancellation.clone(),
            checkpoint: Some(LlmGenerationCheckpoint {
                writer: &stream.checkpoint_writer,
                session: &stream.session,
                metadata: GenerationCheckpointMetadata {
                    step_key: Some("outline_generation".to_string()),
                    call_key: Some(call_key.to_string()),
                    provider_model: Some(context.llm_model_name()),
                    ..Default::default()
                },
            }),
            usage_base: stream.session.snapshot().usage,
        },
    )
    .await
    .map_err(map_generation_error)?;
    Ok(ChatResponse {
        message: ChatMessage::assistant(output.content),
        usage: match output.usage.as_ref() {
            Some(usage) => {
                Some(provider_usage(usage).map_err(|error| AppError::LlmError(error.message))?)
            }
            None => None,
        },
        finish_reason: Some(generation_finish_reason_text(output.finish_reason)),
    })
}

fn map_generation_error(error: GenerationError) -> AppError {
    if error.code.as_str() == "generation_interrupted" {
        return AppError::Cancelled;
    }
    if error.code.as_str() == "llm_output_truncated" {
        return AppError::InvalidArgument(error.message);
    }
    AppError::LlmError(error.message)
}
