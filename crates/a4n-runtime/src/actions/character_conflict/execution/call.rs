use crate::{
    AppState,
    actions::workflow_checkpoint::{
        CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
        required_llm_call_checkpoint_json,
    },
};
use a4n_core::error::AppError;
use a4n_llm::provider::is_truncated_finish_reason;
use serde_json::{Value, json};

use super::rules::character_conflict_call_fingerprint_json;
use crate::actions::character_conflict::{
    CharacterConflictProgressReporter, CharacterConflictRequestContext,
    parsing::{parse_conflict_output_with_source_ids, restored_character_conflict_checkpoint},
};

pub(super) async fn run_character_conflict_llm_call(
    state: &AppState,
    workflow_run_id: i64,
    workflow_fingerprint: Value,
    mut context: CharacterConflictRequestContext,
    progress: CharacterConflictProgressReporter<'_>,
) -> Result<Vec<Value>, AppError> {
    let call_key = context.call_key.clone();
    let fingerprint_json = character_conflict_call_fingerprint_json(
        &call_key,
        &workflow_fingerprint,
        &context.request,
    )?;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
    if let Some(trace_context) = context.request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await?;
    if let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &fingerprint_json)
        .await?
    {
        progress(3, 4, "角色冲突检查已从断点恢复，正在解析矛盾项")?;
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
        return restored_character_conflict_checkpoint(
            checkpoint_json,
            &call_key,
            &context.source_ids,
        );
    }

    let response = state
        .llm
        .chat(context.request)
        .await
        .map_err(|err| AppError::LlmError(err.to_string()))?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(
            "角色冲突检查输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次检查上下文后重试".to_string(),
        ));
    }
    progress(3, 4, "模型已返回，正在解析矛盾项")?;
    let conflicts =
        parse_conflict_output_with_source_ids(&response.message.content, &context.source_ids)?;
    let response_usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    state
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: 1,
            input_fingerprint_json: fingerprint_json,
            checkpoint_json: json!({
                "output": conflicts.clone(),
                "parsed_output": conflicts.clone(),
                "response_content": response.message.content,
                "finish_reason": response.finish_reason,
                "usage": response_usage_json,
            }),
            artifacts_json: None,
        })
        .await?;
    Ok(conflicts)
}
