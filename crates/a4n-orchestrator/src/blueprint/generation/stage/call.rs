use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use serde_json::{Value, json};

use crate::blueprint::OrchestratorContext;
use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};

use super::super::checkpoint::restored_blueprint_stage_output;
use super::contracts::PreparedBlueprintStage;
use super::repair::repair_blueprint_stage_output;
use super::rules::{blueprint_stage_call_order, ensure_not_cancelled, report};
use super::transport::generate_blueprint_stage_output;

pub(super) async fn execute_blueprint_stage_call(
    context: &OrchestratorContext,
    prepared: PreparedBlueprintStage<'_>,
) -> Result<Value, AppError> {
    if let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        context.db.connection(),
        prepared.workflow_run_id,
        &prepared.call_key,
        &prepared.fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?
    {
        let checkpoint_json =
            workflow::required_llm_call_checkpoint_json(checkpoint, &prepared.call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let value = restored_blueprint_stage_output(
            checkpoint_json,
            prepared.spec.title,
            prepared.spec.expected_top_level_keys,
        )?;
        (prepared.spec.validate)(&value)?;
        report(
            prepared.progress,
            prepared.spec.progress_current,
            prepared.spec.restored_message,
        )?;
        return Ok(value);
    }

    let output = generate_blueprint_stage_output(
        context,
        prepared.request.clone(),
        prepared.spec,
        prepared.cancel_token,
    )
    .await?;
    ensure_not_cancelled(prepared.cancel_token)?;
    let call_order = blueprint_stage_call_order(prepared.spec.progress_current)?;
    let repaired = repair_blueprint_stage_output(context, &prepared, call_order, &output).await?;
    workflow::save_completed_llm_call_checkpoint(
        context.db.connection(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id: prepared.workflow_run_id,
            call_key: prepared.call_key,
            call_order,
            input_fingerprint_json: prepared.fingerprint_json,
            checkpoint_json: json!({
                "output": repaired.final_value.clone(),
                "parsed_output": repaired.final_value.clone(),
                "response_content": output.response_content,
                "usage": output.usage,
                "finish_reason": output.finish_reason,
            }),
            artifacts_json: Some(json!({
                "structured_repair": repaired.artifact_summary(),
            })),
        },
    )
    .await
    .map_err(workflow_write_app_error)?;
    Ok(repaired.output)
}
