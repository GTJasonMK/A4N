use std::sync::Arc;

use a4n_domain::project::{BLUEPRINT_RAW_JSON_STREAM_ID, blueprint_stage_call_key};
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::blueprint_generation::ensure_blueprint_response_not_truncated;
use tokio_util::sync::CancellationToken;

use crate::blueprint::OrchestratorContext;
use crate::generation::{
    checkpoint::{GenerationCheckpointMetadata, GenerationCheckpointWriter},
    events::{GenerationEventPublisher, GenerationTaskSession},
    generation_finish_reason_text,
    stream::{LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation},
};

use super::super::contracts::BlueprintStageSpec;
use super::contracts::BlueprintStageLlmOutput;
use super::rules::ensure_not_cancelled;

pub(super) async fn generate_blueprint_stage_output(
    context: &OrchestratorContext,
    request: ChatRequest,
    spec: &BlueprintStageSpec,
    publisher: &dyn GenerationEventPublisher,
    session: &Arc<GenerationTaskSession>,
    checkpoint_writer: &GenerationCheckpointWriter,
    cancel_token: CancellationToken,
) -> Result<BlueprintStageLlmOutput, AppError> {
    ensure_not_cancelled(&cancel_token)?;
    let call_key = blueprint_stage_call_key(spec.key);
    let output = execute_llm_generation(
        context.llm_backend(),
        publisher,
        LlmGenerationRequest {
            request,
            stream_id: BLUEPRINT_RAW_JSON_STREAM_ID.to_string(),
            cancellation: cancel_token,
            checkpoint: Some(LlmGenerationCheckpoint {
                writer: checkpoint_writer,
                session,
                metadata: GenerationCheckpointMetadata {
                    step_key: Some(call_key.clone()),
                    call_key: Some(call_key),
                    provider_model: Some(context.llm_model_name()),
                    ..Default::default()
                },
            }),
            usage_base: session.snapshot().usage,
        },
    )
    .await
    .map_err(|error| {
        if error.code.as_str() == "generation_interrupted" {
            AppError::Cancelled
        } else {
            AppError::LlmError(error.message)
        }
    })?;
    let finish_reason = generation_finish_reason_text(output.finish_reason);
    ensure_blueprint_response_not_truncated(Some(finish_reason.as_str()), spec.title)?;
    Ok(BlueprintStageLlmOutput {
        response_content: output.content,
        usage: output.usage.map(serde_json::to_value).transpose()?,
        finish_reason: Some(finish_reason),
    })
}
