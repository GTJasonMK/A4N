use a4n_core::{
    error::AppError,
    models::{ai::optimization_session, chapter::chapter},
};
use a4n_db::workflow;
use a4n_llm::provider::{ChatMessage, ChatResponse};
use serde_json::Value;

use crate::{
    optimize::{
        OrchestratorContext,
        patch::restore_content_optimization_patch_proposal,
        response_content_from_checkpoint,
        turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED, TURN_STATUS_FAILED},
    },
    workflow::{workflow_checkpoint_contract_app_error, workflow_read_app_error},
};

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
    let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        service.db.connection(),
        workflow_run_id,
        call_key,
        input_fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?
    else {
        return Ok(None);
    };
    let checkpoint_json = workflow::required_llm_call_checkpoint_json(checkpoint, call_key)
        .map_err(workflow_checkpoint_contract_app_error)?;
    let (response_content, usage, finish_reason) =
        response_content_from_checkpoint(&checkpoint_json, call_key)?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_parsed_output".to_string(),
            operation: format!("读取内容优化 LLM 调用断点 {call_key}"),
        })?;
    let proposal = match restore_content_optimization_patch_proposal(
        parsed_output,
        session.id,
        chapter.project_id,
        chapter.id,
        base_content_hash,
        &chapter.content,
    ) {
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
        },
        proposal,
    }))
}
