use std::sync::Arc;

use a4n_domain::project::ParsedBlueprintOptimization;
use a4n_domain::project::{BLUEPRINT_RAW_JSON_STREAM_ID, BLUEPRINT_REFINEMENT_WORKFLOW_KIND};
use a4n_error::AppError;
use a4n_service_protocol::GenerationFinishReason;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;
use crate::generation::{
    checkpoint::{GenerationCheckpointMetadata, GenerationCheckpointWriter},
    events::{GenerationEventPublisher, GenerationTaskSession},
    generation_finish_reason_from_text, generation_finish_reason_text,
    stream::{LlmGenerationCheckpoint, LlmGenerationRequest},
};
use crate::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use super::super::checkpoint::blueprint_refine_checkpoint_output;
use super::contracts::PreparedBlueprintRefine;

pub(super) struct BlueprintRefineCallOutput {
    pub(super) parsed: ParsedBlueprintOptimization,
    pub(super) usage: Option<a4n_service_protocol::GenerationUsage>,
    pub(super) finish_reason: GenerationFinishReason,
}

pub(super) async fn execute_blueprint_refine_call(
    context: &OrchestratorContext,
    prepared: &PreparedBlueprintRefine,
    publisher: &dyn GenerationEventPublisher,
    session: &Arc<GenerationTaskSession>,
    checkpoint_writer: &GenerationCheckpointWriter,
    cancel_token: CancellationToken,
) -> Result<BlueprintRefineCallOutput, AppError> {
    let checkpoint = context
        .load_completed_llm_call_checkpoint(
            prepared.workflow_run_id,
            &prepared.call_key,
            &prepared.input_fingerprint_json,
        )
        .await?;
    if let Some(checkpoint) = checkpoint {
        if cancel_token.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        for delta in crate::generation::checkpoint::split_text_delta(&checkpoint.response_content) {
            publisher
                .text_delta(BLUEPRINT_RAW_JSON_STREAM_ID, &delta)
                .await
                .map_err(|error| AppError::InvalidState {
                    current: error.to_string(),
                    operation: "恢复蓝图优化流".to_string(),
                })?;
        }
        let restored = blueprint_refine_checkpoint_output(
            &checkpoint.parsed_output,
            &checkpoint.response_content,
            checkpoint.usage,
            checkpoint.finish_reason,
            &prepared.call_key,
        );
        let (parsed, usage_json, finish_reason) = context
            .validate_llm_call_output(
                prepared.workflow_run_id,
                &prepared.call_key,
                &prepared.input_fingerprint_json,
                restored,
            )
            .await?;
        let usage = usage_json
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| AppError::InvalidState {
                current: error.to_string(),
                operation: "恢复蓝图优化 usage".to_string(),
            })?;
        if let Some(usage) = usage.as_ref() {
            crate::generation::stream::publish_generation_usage(
                publisher,
                usage,
                session.snapshot().usage.as_ref(),
            )
            .await
            .map_err(|error| AppError::InvalidState {
                current: error.message,
                operation: "恢复蓝图优化累计 usage".to_string(),
            })?;
        }
        let finish_reason =
            generation_finish_reason_from_text(&finish_reason).ok_or_else(|| {
                AppError::InvalidState {
                    current: finish_reason,
                    operation: "恢复蓝图优化 finish_reason".to_string(),
                }
            })?;
        return Ok(BlueprintRefineCallOutput {
            parsed,
            usage,
            finish_reason,
        });
    }

    let (output, parsed) = crate::generation::stream::execute_validated_llm_generation(
        context.llm_backend(),
        publisher,
        LlmGenerationRequest {
            request: prepared.request.clone(),
            stream_id: BLUEPRINT_RAW_JSON_STREAM_ID.to_string(),
            cancellation: cancel_token.clone(),
            checkpoint: Some(LlmGenerationCheckpoint {
                writer: checkpoint_writer,
                session,
                metadata: GenerationCheckpointMetadata {
                    step_key: Some(BLUEPRINT_REFINEMENT_WORKFLOW_KIND.to_string()),
                    call_key: Some(prepared.call_key.clone()),
                    provider_model: Some(context.llm_model_name()),
                    ..Default::default()
                },
            }),
            usage_base: None,
        },
        "蓝图优化",
        super::super::super::json::parse_blueprint_optimization_json,
    )
    .await
    .map_err(|error| {
        if error.code.as_str() == "generation_interrupted" {
            AppError::Cancelled
        } else {
            AppError::LlmError(error.message)
        }
    })?;
    let response_content = output.content.clone();
    let response_usage_json = output
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let finish_reason_text = generation_finish_reason_text(output.finish_reason);
    let optimization_patch = parsed.raw.clone();
    context
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id: prepared.workflow_run_id,
            call_key: prepared.call_key.clone(),
            call_order: 1,
            input_fingerprint_json: prepared.input_fingerprint_json.clone(),
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: optimization_patch,
                response_content,
                usage: response_usage_json,
                finish_reason: Some(finish_reason_text),
                artifacts_json: None,
            },
        })
        .await?;

    Ok(BlueprintRefineCallOutput {
        parsed,
        usage: output.usage,
        finish_reason: output.finish_reason,
    })
}
