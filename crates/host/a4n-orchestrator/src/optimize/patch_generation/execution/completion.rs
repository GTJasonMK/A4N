use a4n_db_models::models::ai::optimization_session;
use a4n_error::AppError;
use serde_json::{Value, json};

use crate::optimize::runtime_tools::EDIT_ARTICLE_TOOL;
use crate::optimize::{AgentLoopExecutionError, OptimizationPatchProposal, OrchestratorContext};
use crate::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use super::PatchGenerationCallResult;

pub(super) struct PatchGenerationCompletionInput {
    pub(super) requested_session_id: i64,
    pub(super) session: optimization_session::Model,
    pub(super) chapter_id: i64,
    pub(super) turn_id: i64,
    pub(super) base_content_hash: String,
    pub(super) workflow_run_id: i64,
    pub(super) call: PatchGenerationCallResult,
}

pub(in crate::optimize::patch_generation) struct PendingPatchGeneration {
    pub(in crate::optimize::patch_generation) session_id: i64,
    pub(in crate::optimize::patch_generation) chapter_id: i64,
    pub(in crate::optimize::patch_generation) turn_id: i64,
    pub(in crate::optimize::patch_generation) base_content_hash: String,
    pub(in crate::optimize::patch_generation) proposal: OptimizationPatchProposal,
    pub(in crate::optimize::patch_generation) assistant_content: String,
    pub(in crate::optimize::patch_generation) tool_content: String,
    pub(in crate::optimize::patch_generation) tool_calls: Value,
    pub(in crate::optimize::patch_generation) final_reply: String,
}

pub(super) async fn prepare_patch_generation_completion(
    service: &OrchestratorContext,
    input: PatchGenerationCompletionInput,
) -> Result<PendingPatchGeneration, AgentLoopExecutionError> {
    let PatchGenerationCompletionInput {
        requested_session_id,
        session,
        chapter_id,
        turn_id,
        base_content_hash,
        workflow_run_id,
        call,
    } = input;
    let PatchGenerationCallResult {
        response,
        proposal,
        restored,
        call_key,
        input_fingerprint_json,
    } = call;
    let response_content = response.message.content.clone();
    service
        .record_llm_response(session.id, "局部补丁生成", &response.message.content)
        .await?;
    service
        .save_agent_event(session.id, "LLM 已返回局部补丁，正在保存并校验。")
        .await?;
    if !restored {
        let usage = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(AppError::from)?;
        service
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: 1,
                input_fingerprint_json,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: serde_json::to_value(&proposal).map_err(AppError::from)?,
                    response_content: response_content.clone(),
                    usage,
                    finish_reason: response.finish_reason,
                    artifacts_json: None,
                },
            })
            .await?;
    }
    let tool_calls = json!({ "tool": EDIT_ARTICLE_TOOL, "chapter_id": chapter_id });
    let tool_content = serde_json::to_string_pretty(&json!({
        "tool": EDIT_ARTICLE_TOOL,
        "mode": "pending_confirmation",
        "requires_user_confirmation": true,
        "chapter_id": chapter_id,
        "base_content_hash": base_content_hash,
        "patch_count": proposal.patches.len(),
        "summary": &proposal.summary,
        "patches": &proposal.patches,
    }))
    .map_err(AppError::from)?;
    service
        .save_agent_event(
            requested_session_id,
            &format!(
                "局部补丁已校验，共 {} 个，等待 generation terminal 提交。",
                proposal.patches.len()
            ),
        )
        .await?;
    let final_reply = format!(
        "已生成 {} 个局部补丁提案，等待你在正文 diff 中审阅。",
        proposal.patches.len()
    );
    Ok(PendingPatchGeneration {
        session_id: requested_session_id,
        chapter_id,
        turn_id,
        base_content_hash,
        proposal,
        assistant_content: response_content,
        tool_content,
        tool_calls,
        final_reply,
    })
}
