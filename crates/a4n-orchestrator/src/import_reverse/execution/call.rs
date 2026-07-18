use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use a4n_llm::provider::is_truncated_finish_reason;
use serde_json::{Value, json};

use crate::OrchestratorContext;
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};
use crate::workflow::{workflow_read_app_error, workflow_write_app_error};

use super::checkpoint::restored_import_reverse_checkpoint;
use super::contracts::PreparedImportReverseCall;
use super::schema::reverse_engineer_output_schema;

pub(super) async fn execute_import_reverse_call(
    context: &OrchestratorContext,
    prepared: PreparedImportReverseCall,
) -> Result<Value, AppError> {
    let PreparedImportReverseCall {
        prompt_name,
        request,
        workflow_run_id,
        call_key,
        fingerprint_json,
    } = prepared;
    if let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        context.db.connection(),
        workflow_run_id,
        &call_key,
        &fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?
    {
        let checkpoint_json = workflow::required_llm_call_checkpoint_json(checkpoint, &call_key)
            .map_err(crate::workflow::workflow_checkpoint_contract_app_error)?;
        return restored_import_reverse_checkpoint(checkpoint_json, &prompt_name);
    }

    let response = context.llm.chat(request.clone()).await?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(format!(
            "{prompt_name} 输出达到当前导入反推 max_tokens 上限，返回 JSON 被截断；请在设置页调大对应输出档位，或缩短导入样本后重试"
        )));
    }
    let parse_output = |value: &Value| {
        crate::import_reverse::application::validate_reverse_engineer_output(&prompt_name, value)?;
        Ok(value.clone())
    };
    let repaired = context
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request,
            raw_response: &response.message.content,
            label: &prompt_name,
            schema: reverse_engineer_output_schema(&prompt_name)?,
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id,
                base_call_key: call_key.clone(),
                call_order: 1,
            }),
            cancel_token: None,
            progress: None,
            parse_output: &parse_output,
        })
        .await?;
    let repair_artifact = repaired.artifact_summary();
    let output = repaired.output;
    let usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    workflow::save_completed_llm_call_checkpoint(
        context.db.connection(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: 1,
            input_fingerprint_json: fingerprint_json,
            checkpoint_json: json!({
                "output": output.clone(),
                "parsed_output": output.clone(),
                "response_content": response.message.content,
                "usage": usage_json,
                "finish_reason": response.finish_reason,
            }),
            artifacts_json: Some(json!({
                "structured_repair": repair_artifact,
            })),
        },
    )
    .await
    .map_err(workflow_write_app_error)?;
    Ok(output)
}
