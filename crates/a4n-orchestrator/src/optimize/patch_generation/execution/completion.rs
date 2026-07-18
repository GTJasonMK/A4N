use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use a4n_db::optimize::OptimizationSessionUpdateInput;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use serde_json::json;

use crate::optimize::runtime_tools::EDIT_ARTICLE_TOOL;
use crate::optimize::streaming::{OptimizeProgress, report_optimize_progress};
use crate::optimize::turns::{
    EVENT_KIND_ASSISTANT_UPDATE, EVENT_KIND_TOOL_RESULT, EVENT_STATUS_COMPLETED,
};
use crate::optimize::{OptimizationPatchProposal, OrchestratorContext, optimize_db_error};
use crate::workflow::workflow_write_app_error;

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

pub(super) async fn complete_patch_generation(
    service: &OrchestratorContext,
    input: PatchGenerationCompletionInput,
    progress: &mut Option<OptimizeProgress<'_>>,
) -> Result<OptimizationPatchProposal, AppError> {
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
    service
        .record_llm_response(session.id, "局部补丁生成", &response.message.content)
        .await?;
    service
        .save_agent_event(session.id, "LLM 已返回局部补丁，正在保存并校验。")
        .await?;
    service
        .save_message(session.id, "assistant", &response.message.content)
        .await?;
    report_optimize_progress(progress, 3, 4, "模型已返回，正在校验补丁")?;
    if !restored {
        let usage = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        workflow::save_completed_llm_call_checkpoint(
            service.db.connection(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: call_key.clone(),
                call_order: 1,
                input_fingerprint_json,
                checkpoint_json: json!({
                    "output": proposal.clone(),
                    "parsed_output": proposal.clone(),
                    "response_content": response.message.content,
                    "usage": usage,
                    "finish_reason": response.finish_reason,
                }),
                artifacts_json: None,
            },
        )
        .await
        .map_err(workflow_write_app_error)?;
    }
    let patch_set = service.persist_patch_proposal(turn_id, &proposal).await?;
    service
        .save_message_with_tool_calls(
            session.id,
            "tool",
            &serde_json::to_string_pretty(&json!({
                "tool": EDIT_ARTICLE_TOOL,
                "mode": "pending_confirmation",
                "requires_user_confirmation": true,
                "chapter_id": chapter_id,
                "base_content_hash": base_content_hash,
                "patch_count": proposal.patches.len(),
                "summary": &proposal.summary,
                "patches": &proposal.patches,
            }))?,
            Some(json!({ "tool": EDIT_ARTICLE_TOOL, "chapter_id": chapter_id })),
        )
        .await?;
    if let Some(patch_set) = patch_set {
        service
            .append_turn_event(
                turn_id,
                EVENT_KIND_TOOL_RESULT,
                EVENT_STATUS_COMPLETED,
                &format!("已生成 {} 个局部补丁，等待审阅。", proposal.patches.len()),
                Some(json!({
                    "tool": EDIT_ARTICLE_TOOL,
                    "patch_set_id": patch_set.id,
                    "patch_count": proposal.patches.len(),
                    "summary": &proposal.summary,
                })),
            )
            .await?;
    } else {
        service
            .append_turn_event(
                turn_id,
                EVENT_KIND_ASSISTANT_UPDATE,
                EVENT_STATUS_COMPLETED,
                "没有生成可安全唯一定位的局部补丁。",
                Some(json!({ "tool": EDIT_ARTICLE_TOOL, "summary": &proposal.summary })),
            )
            .await?;
    }

    a4n_db::optimize::update_optimization_session(
        service.db.connection(),
        OptimizationSessionUpdateInput {
            session,
            status: None,
            issues: None,
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: None,
            summary: Some(Some(format!(
                "已生成 {} 个局部补丁提案，等待用户审阅后应用",
                proposal.patches.len()
            ))),
        },
    )
    .await
    .map_err(optimize_db_error)?;
    service
        .save_agent_event(
            requested_session_id,
            &format!("局部补丁校验通过，共 {} 个。", proposal.patches.len()),
        )
        .await?;
    report_optimize_progress(progress, 4, 4, "局部补丁已生成")?;
    service
        .finish_turn(
            turn_id,
            &format!(
                "已生成 {} 个局部补丁提案，等待你在正文 diff 中审阅。",
                proposal.patches.len()
            ),
        )
        .await?;

    Ok(proposal)
}
