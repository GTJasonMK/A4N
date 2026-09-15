use a4n_api_contract::import_reverse::ReverseEngineeringCandidate;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::is_truncated_finish_reason;
use serde_json::{Value, json};

use crate::OrchestratorContext;
use crate::structured_llm::parse_strict_json_response;
use crate::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use super::checkpoint::restored_import_reverse_checkpoint;
use super::contracts::PreparedImportReverseCall;

pub(super) async fn execute_import_reverse_call(
    context: &OrchestratorContext,
    prepared: PreparedImportReverseCall,
) -> Result<ReverseEngineeringCandidate, AppError> {
    let PreparedImportReverseCall {
        target,
        request,
        workflow_run_id,
        fingerprint_json,
    } = prepared;
    let prompt_name = target.prompt_name();
    if let Some(checkpoint) = context
        .load_completed_llm_call_checkpoint(workflow_run_id, prompt_name, &fingerprint_json)
        .await?
    {
        let output = restored_import_reverse_checkpoint(
            &checkpoint.parsed_output,
            &checkpoint.response_content,
            target,
        )?;
        return crate::import_reverse::candidate::reverse_candidate_checkpoint_response(
            workflow_run_id,
            target,
            output,
            checkpoint.artifacts_json.as_ref(),
        );
    }

    let response = context.stream_complete_llm(request).await?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(format!(
            "{prompt_name} 输出达到当前导入反推 max_tokens 上限，返回 JSON 被截断；请在模型配置中提高 max_output_tokens，或缩短导入样本后重试"
        )));
    }
    let output: Value = parse_strict_json_response(&response.message.content, prompt_name)?;
    crate::import_reverse::application::validate_reverse_engineer_output(target, &output)?;
    let output_fingerprint_sha256 =
        a4n_db::generation::generation_checkpoint_fingerprint_sha256(&output)
            .map_err(AppError::Serialization)?;
    let candidate =
        crate::import_reverse::candidate::reverse_candidate_proposal(&output_fingerprint_sha256);
    let candidate_response =
        crate::import_reverse::candidate::reverse_candidate_checkpoint_response(
            workflow_run_id,
            target,
            output.clone(),
            Some(&json!({"candidate": candidate.clone()})),
        )?;
    let usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    context
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: prompt_name.to_string(),
            call_order: 1,
            input_fingerprint_json: fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: output,
                response_content: response.message.content,
                usage: usage_json,
                finish_reason: response.finish_reason,
                artifacts_json: Some(json!({
                    "candidate": candidate,
                })),
            },
        })
        .await?;
    Ok(candidate_response)
}
