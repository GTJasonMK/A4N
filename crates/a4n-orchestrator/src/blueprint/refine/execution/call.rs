use std::sync::Arc;

use a4n_core::{blueprint::ParsedBlueprintOptimization, error::AppError};
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use a4n_service_protocol::GenerationFinishReason;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;
use crate::generation::{
    checkpoint::{GenerationCheckpointMetadata, GenerationCheckpointWriter},
    events::{GenerationEventPublisher, GenerationTaskSession},
    generation_finish_reason_text,
    stream::{LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation},
};
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};
use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};

use super::super::checkpoint::blueprint_refine_checkpoint_output;
use super::contracts::PreparedBlueprintRefine;
use super::schema::blueprint_refine_output_schema;

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
    let checkpoint = workflow::load_completed_llm_call_checkpoint(
        context.database.connection(),
        prepared.workflow_run_id,
        &prepared.call_key,
        &prepared.input_fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?;
    if let Some(checkpoint) = checkpoint {
        let checkpoint_json =
            workflow::required_llm_call_checkpoint_json(checkpoint, &prepared.call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let response_content = checkpoint_json
            .get("response_content")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("{}.response_content 缺失", prepared.call_key),
                operation: "恢复蓝图优化响应".to_string(),
            })?;
        publisher
            .text_delta("raw_json", response_content)
            .await
            .map_err(|error| AppError::InvalidState {
                current: error.to_string(),
                operation: "恢复蓝图优化流".to_string(),
            })?;
        let (parsed, usage_json, finish_reason) =
            blueprint_refine_checkpoint_output(&checkpoint_json, &prepared.call_key)?;
        let usage = usage_json
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| AppError::InvalidState {
                current: error.to_string(),
                operation: "恢复蓝图优化 usage".to_string(),
            })?;
        let finish_reason = finish_reason
            .as_deref()
            .map(|value| match value {
                "stop" => Ok(GenerationFinishReason::Stop),
                "tool_calls" => Ok(GenerationFinishReason::ToolCallsComplete),
                other => Err(AppError::InvalidState {
                    current: other.to_string(),
                    operation: "恢复蓝图优化 finish_reason".to_string(),
                }),
            })
            .transpose()?
            .unwrap_or(GenerationFinishReason::Stop);
        return Ok(BlueprintRefineCallOutput {
            parsed,
            usage,
            finish_reason,
        });
    }

    let output = execute_llm_generation(
        context.llm_backend(),
        publisher,
        LlmGenerationRequest {
            request: prepared.request.clone(),
            stream_id: "raw_json".to_string(),
            cancellation: cancel_token.clone(),
            checkpoint: Some(LlmGenerationCheckpoint {
                writer: checkpoint_writer,
                session,
                metadata: GenerationCheckpointMetadata {
                    step_key: Some("blueprint_refinement".to_string()),
                    call_key: Some(prepared.call_key.clone()),
                    provider_model: Some(context.llm_model_name()),
                    ..Default::default()
                },
            }),
            usage_base: None,
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
    let response_content = output.content.clone();
    let response_usage_json = output
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let finish_reason_text = generation_finish_reason_text(output.finish_reason);
    let parse_output = |value: &Value| {
        super::super::super::json::parse_blueprint_optimization_json(&serde_json::to_string(value)?)
    };
    let repaired = context
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request: prepared.request.clone(),
            raw_response: &response_content,
            label: "蓝图优化",
            schema: blueprint_refine_output_schema()?,
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id: prepared.workflow_run_id,
                base_call_key: prepared.call_key.clone(),
                call_order: 1,
            }),
            cancel_token: Some(&cancel_token),
            progress: None,
            parse_output: &parse_output,
        })
        .await?;
    let repair_artifacts = repaired.artifact_summary();
    let parsed = repaired.output;
    let optimization_patch = parsed.raw.clone();
    workflow::save_completed_llm_call_checkpoint(
        context.database.connection(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id: prepared.workflow_run_id,
            call_key: prepared.call_key.clone(),
            call_order: 1,
            input_fingerprint_json: prepared.input_fingerprint_json.clone(),
            checkpoint_json: json!({
                "output": optimization_patch.clone(),
                "parsed_output": optimization_patch,
                "response_content": response_content,
                "usage": response_usage_json,
                "finish_reason": finish_reason_text,
            }),
            artifacts_json: Some(json!({
                "structured_repair": repair_artifacts,
            })),
        },
    )
    .await
    .map_err(workflow_write_app_error)?;

    Ok(BlueprintRefineCallOutput {
        parsed,
        usage: output.usage,
        finish_reason: output.finish_reason,
    })
}
