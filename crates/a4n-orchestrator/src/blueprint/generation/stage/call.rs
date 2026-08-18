use a4n_core::{
    blueprint::{BLUEPRINT_RAW_JSON_STREAM_ID, blueprint_stage_call_key},
    error::AppError,
};
use a4n_domain::progress::{ProgressPhaseKind, ProgressSeverity};
use a4n_prompt::blueprint_generation::parse_blueprint_stage_json_object;

use crate::blueprint::OrchestratorContext;
use crate::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use super::super::checkpoint::restored_blueprint_stage_output;
use super::contracts::{BlueprintStageExecution, PreparedBlueprintStage};
use super::rules::ensure_not_cancelled;
use super::transport::generate_blueprint_stage_output;

pub(super) async fn execute_blueprint_stage_call(
    context: &OrchestratorContext,
    prepared: PreparedBlueprintStage<'_>,
) -> Result<BlueprintStageExecution, AppError> {
    let PreparedBlueprintStage {
        spec,
        publisher,
        session,
        checkpoint_writer,
        cancel_token,
        request,
        workflow_run_id,
        fingerprint_json,
    } = prepared;
    let call_key = blueprint_stage_call_key(spec.key);
    if let Some(checkpoint) = context
        .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &fingerprint_json)
        .await?
    {
        publisher
            .text_delta(BLUEPRINT_RAW_JSON_STREAM_ID, &checkpoint.response_content)
            .await
            .map_err(|error| AppError::InvalidState {
                current: error.to_string(),
                operation: "恢复蓝图阶段原始输出".to_string(),
            })?;
        let value = restored_blueprint_stage_output(
            &checkpoint.parsed_output,
            &checkpoint.response_content,
            spec.title,
            spec.output_schema,
        )?;
        (spec.validate)(&value)?;
        publisher
            .progress(a4n_service_protocol::ProgressEvent {
                stage_code: spec.key.to_string(),
                phase_kind: ProgressPhaseKind::Parse,
                message: spec.restored_message.to_string(),
                current: Some(u64::from(spec.progress_current)),
                total: Some(a4n_core::blueprint::BLUEPRINT_GENERATE_PROGRESS_TOTAL as u64),
                unit_label: None,
                item_key: None,
                item_index: None,
                item_total: None,
                severity: ProgressSeverity::Info,
                estimated: false,
            })
            .await
            .map_err(|error| AppError::InvalidState {
                current: error.to_string(),
                operation: "发布蓝图阶段恢复进度".to_string(),
            })?;
        return Ok(BlueprintStageExecution {
            output: value,
            request,
        });
    }

    let output = generate_blueprint_stage_output(
        context,
        request.clone(),
        spec,
        publisher,
        session,
        checkpoint_writer,
        cancel_token.clone(),
    )
    .await?;
    ensure_not_cancelled(&cancel_token)?;
    let parsed = parse_blueprint_stage_json_object(
        &output.response_content,
        spec.title,
        spec.output_schema,
    )?;
    (spec.validate)(&parsed)?;
    context
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: i32::from(spec.progress_current),
            input_fingerprint_json: fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: parsed.clone(),
                response_content: output.response_content,
                usage: output.usage,
                finish_reason: output.finish_reason,
                artifacts_json: None,
            },
        })
        .await?;
    Ok(BlueprintStageExecution {
        output: parsed,
        request,
    })
}
