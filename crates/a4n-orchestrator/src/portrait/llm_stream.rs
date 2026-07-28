//! 人物立绘 generation 任务内的统一 LLM stream 上下文。

use std::future::Future;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatMessage, ChatRequest, ChatResponse};
use a4n_service_protocol::GenerationError;
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

task_local! {
    static PORTRAIT_LLM_STREAM: PortraitLlmStreamContext;
}

#[derive(Clone)]
pub(in crate::portrait) struct PortraitLlmStreamContext {
    pub publisher: SessionGenerationEventPublisher,
    pub session: Arc<GenerationTaskSession>,
    pub checkpoint_writer: GenerationCheckpointWriter,
    pub cancellation: CancellationToken,
}

pub(in crate::portrait) async fn with_portrait_llm_stream<F, Fut, T>(
    context: PortraitLlmStreamContext,
    work: F,
) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    PORTRAIT_LLM_STREAM.scope(context, work()).await
}

pub(in crate::portrait) async fn execute_portrait_llm_call(
    context: &OrchestratorContext,
    request: ChatRequest,
    call_key: &str,
) -> Result<ChatResponse, AppError> {
    let stream = PORTRAIT_LLM_STREAM
        .try_with(|value| value.clone())
        .map_err(|_| AppError::InvalidState {
            current: "portrait_llm_stream_missing".to_string(),
            operation: format!(
                "人物立绘 LLM 调用 {call_key} 必须在 generation stream 上下文中执行"
            ),
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
                    step_key: Some("portrait_generation".to_string()),
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
