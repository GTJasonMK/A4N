use a4n_core::{blueprint::ParsedBlueprintOptimization, error::AppError};
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use a4n_prompt::blueprint_generation::ensure_blueprint_response_not_truncated;
use serde_json::{Value, json};

use crate::OrchestratorContext;
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};
use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};

use super::super::checkpoint::blueprint_refine_checkpoint_output;
use super::contracts::PreparedBlueprintRefine;
use super::schema::blueprint_refine_output_schema;

pub(super) async fn execute_blueprint_refine_call(
    context: &OrchestratorContext,
    prepared: &PreparedBlueprintRefine,
) -> Result<ParsedBlueprintOptimization, AppError> {
    let checkpoint = workflow::load_completed_llm_call_checkpoint(
        context.db.connection(),
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
        let (parsed, _, _) =
            blueprint_refine_checkpoint_output(&checkpoint_json, &prepared.call_key)?;
        return Ok(parsed);
    }

    let response = context.llm.chat(prepared.request.clone()).await?;
    ensure_blueprint_response_not_truncated(response.finish_reason.as_deref(), "蓝图优化")?;
    let checkpoint_response_text = response.message.content.clone();
    let response_usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let response_finish_reason = response.finish_reason.clone();
    let parse_output = |value: &Value| {
        super::super::super::json::parse_blueprint_optimization_json(&serde_json::to_string(value)?)
    };
    let repaired = context
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request: prepared.request.clone(),
            raw_response: &checkpoint_response_text,
            label: "蓝图优化",
            schema: blueprint_refine_output_schema()?,
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id: prepared.workflow_run_id,
                base_call_key: prepared.call_key.clone(),
                call_order: 1,
            }),
            cancel_token: None,
            progress: None,
            parse_output: &parse_output,
        })
        .await?;
    let parsed = repaired.output;
    let optimization_patch = parsed.raw.clone();
    workflow::save_completed_llm_call_checkpoint(
        context.db.connection(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id: prepared.workflow_run_id,
            call_key: prepared.call_key.clone(),
            call_order: 1,
            input_fingerprint_json: prepared.input_fingerprint_json.clone(),
            checkpoint_json: json!({
                "output": optimization_patch.clone(),
                "parsed_output": optimization_patch,
                "response_content": checkpoint_response_text,
                "usage": response_usage_json,
                "finish_reason": response_finish_reason,
            }),
            artifacts_json: None,
        },
    )
    .await
    .map_err(workflow_write_app_error)?;

    Ok(parsed)
}
