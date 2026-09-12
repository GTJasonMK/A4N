use a4n_db_models::models::{ai::optimization_session, chapter::chapter};
use a4n_error::AppError;
use a4n_llm::provider::{ChatMessage, ChatResponse};
use serde_json::Value;

use crate::optimize::{
    OrchestratorContext,
    patch::parse_content_optimization_patch_response,
    turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED, TURN_STATUS_FAILED},
};
use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

use super::contracts::RestoredPatchGenerationCall;

pub(super) async fn restore_patch_generation_checkpoint(
    service: &OrchestratorContext,
    session: &optimization_session::Model,
    chapter: &chapter::Model,
    turn_id: i64,
    base_content_hash: &str,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<RestoredPatchGenerationCall>, AppError> {
    let Some(checkpoint) = service
        .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
        .await?
    else {
        return Ok(None);
    };
    let response_content = checkpoint.response_content;
    let usage = checkpoint.usage;
    let finish_reason = checkpoint.finish_reason;
    let parsed_output = checkpoint.parsed_output;
    let proposal = match (|| {
        let proposal = parse_content_optimization_patch_response(
            session.id,
            chapter.project_id,
            chapter.id,
            base_content_hash,
            &chapter.content,
            &response_content,
        )?;
        let reparsed_output = serde_json::to_value(&proposal)?;
        ensure_checkpoint_parsed_output_matches(
            &parsed_output,
            &reparsed_output,
            &format!("内容优化局部补丁 {call_key}"),
        )?;
        Ok::<_, AppError>(proposal)
    })() {
        Ok(proposal) => proposal,
        Err(err) => {
            service
                .append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("局部补丁校验失败：{err}"),
                    None,
                )
                .await?;
            service
                .fail_turn(turn_id, TURN_STATUS_FAILED, &err.to_string())
                .await?;
            return Err(err);
        }
    };
    service
        .save_agent_event(session.id, "已恢复局部补丁 LLM 调用断点。")
        .await?;
    Ok(Some(RestoredPatchGenerationCall {
        response: ChatResponse {
            message: ChatMessage::assistant(response_content),
            usage: usage
                .map(serde_json::from_value)
                .transpose()
                .map_err(|err| AppError::InvalidState {
                    current: "invalid_checkpoint_usage".to_string(),
                    operation: format!("恢复内容优化 LLM 调用断点 {call_key}: {err}"),
                })?,
            finish_reason,
            reasoning_content: None,
        },
        proposal,
    }))
}
