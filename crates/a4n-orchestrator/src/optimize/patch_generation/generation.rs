use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, optimize::StartOptimizePatchGenerationInput,
};
use a4n_core::error::AppError;
use a4n_db::optimize::{OptimizationPatchCreateInput, OptimizationPatchSetCreateInput};
use a4n_domain::progress::ProgressPhaseKind;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::{
    GenerationFinishReason, GenerationResultRef, GenerationTaskStatus, StreamTextHash,
};
use futures::future::BoxFuture;
use sha2::{Digest, Sha256};

use crate::{
    OrchestratorContext,
    context::GenerationTaskExecution,
    generation::{
        checkpoint::GenerationCheckpointMetadata, events::GenerationEventPublisher,
        generation_session_error, generation_terminal_error, terminal::CompletionDescriptor,
    },
    optimize::{
        AgentLoopGenerationContext, OptimizeProposePatchesInput,
        agent_loop_error_to_generation_error, app_error_to_generation_error,
        generation_stream_completion, is_generation_cancelled, optimize_db_error,
        optimize_progress,
    },
    workflow::WorkflowRunFinalizeOutcome,
};

use super::execution::{PendingPatchGeneration, prepare_patch_generation_result};

pub struct OptimizePatchGenerationApplicationContext<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
}

impl OrchestratorContext {
    pub async fn run_optimize_patch_generation_task(
        &self,
        context: OptimizePatchGenerationApplicationContext<'_>,
        user_id: i64,
        input: StartOptimizePatchGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        let Some(execution) = self
            .acquire_generation_task_execution(&handle)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let session = execution.session.clone();
        let cancel_token = execution.permit.cancellation_token();
        let publisher = execution.publisher.clone();
        let checkpoint_writer = execution.checkpoint_writer.clone();
        let request = input.request;

        let optimize_session = match self
            .optimize_session_for_user(user_id, request.session_id)
            .await
        {
            Ok(session) => session,
            Err(error) => {
                return fail_optimize_patch_generation_execution(
                    self,
                    execution,
                    request.session_id,
                    None,
                    None,
                    app_error_to_generation_error(error, "optimization_patch"),
                )
                .await;
            }
        };
        if let Err(error) = publisher
            .progress(optimize_progress(
                "optimization_patch_prepare",
                ProgressPhaseKind::Prepare,
                "正在准备局部修复补丁",
            ))
            .await
        {
            return fail_optimize_patch_generation_execution(
                self,
                execution,
                request.session_id,
                None,
                None,
                generation_session_error(error),
            )
            .await;
        }
        let workflow = match self
            .begin_optimize_session_workflow_validated(
                optimize_session.id,
                "content_optimization_patch",
                "patch",
                optimize_session.project_id,
                optimize_session.chapter_id,
                Some(handle.task_id.clone()),
            )
            .await
        {
            Ok(workflow) => workflow,
            Err(error) => {
                return fail_optimize_patch_generation_execution(
                    self,
                    execution,
                    request.session_id,
                    None,
                    None,
                    app_error_to_generation_error(error, "optimization_patch_workflow"),
                )
                .await;
            }
        };

        let generation = AgentLoopGenerationContext {
            publisher: &publisher,
            cancellation: &cancel_token,
            checkpoint_writer: &checkpoint_writer,
            session: &session,
        };
        let pending = match prepare_patch_generation_result(
            self,
            OptimizeProposePatchesInput {
                session_id: request.session_id,
                issue_ids: request.issue_ids,
                prompt_renderer: context.prompt_renderer,
                agent_instruction_override: request.agent_instruction.as_deref(),
                workflow_run_id: workflow.id,
                generation: &generation,
            },
        )
        .await
        {
            Ok(pending) => pending,
            Err(error) => {
                return fail_optimize_patch_generation_execution(
                    self,
                    execution,
                    request.session_id,
                    Some(workflow.id),
                    None,
                    agent_loop_error_to_generation_error(error),
                )
                .await;
            }
        };

        if cancel_token.is_cancelled()
            || session.snapshot().status == GenerationTaskStatus::Cancelling
        {
            return fail_optimize_patch_generation_execution(
                self,
                execution,
                request.session_id,
                Some(workflow.id),
                Some(pending.turn_id),
                crate::generation::generation_public_error(
                    "generation_interrupted",
                    "provider_stream",
                    "局部补丁生成已取消",
                ),
            )
            .await;
        }
        if let Err(error) = checkpoint_writer
            .persist_if_due(
                &session,
                GenerationCheckpointMetadata {
                    step_key: Some("optimization_patch_generation".to_string()),
                    provider_model: Some(self.llm_model_name()),
                    ..Default::default()
                },
                true,
            )
            .await
        {
            return fail_optimize_patch_generation_execution(
                self,
                execution,
                request.session_id,
                Some(workflow.id),
                Some(pending.turn_id),
                crate::generation::generation_public_error(
                    "generation_checkpoint_unavailable",
                    "checkpoint",
                    error,
                ),
            )
            .await;
        }
        let stream = match generation_stream_completion(&session) {
            Ok(stream) => stream,
            Err(error) => {
                return fail_optimize_patch_generation_execution(
                    self,
                    execution,
                    request.session_id,
                    Some(workflow.id),
                    Some(pending.turn_id),
                    error,
                )
                .await;
            }
        };
        let patch_set = patch_set_input(&pending);
        let session_id = pending.session_id;
        let turn_id = pending.turn_id;
        let assistant_content = pending.assistant_content.clone();
        let tool_content = pending.tool_content.clone();
        let tool_calls = pending.tool_calls.clone();
        let final_reply = pending.final_reply.clone();
        let session_summary = format!(
            "已生成 {} 个局部补丁提案，等待用户审阅后应用",
            pending.proposal.patches.len()
        );
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                let input = a4n_db::optimize::CompleteOptimizationPatchTurnInput {
                    session_id,
                    turn_id,
                    assistant_content: assistant_content.clone(),
                    tool_content: tool_content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    patch_set,
                    final_reply: final_reply.clone(),
                    session_summary,
                    tool_name: "edit_article".to_string(),
                };
                let stream = stream;
                Box::pin(async move {
                    let result = a4n_db::optimize::complete_optimization_patch_turn(tx, input)
                        .await
                        .map_err(optimize_db_error)?;
                    Ok(CompletionDescriptor {
                        result_ref: Some(GenerationResultRef {
                            kind: "optimization_turn".to_string(),
                            id: result.turn.id.to_string(),
                            revision: format!("{:x}", Sha256::digest(final_reply.as_bytes())),
                        }),
                        final_text_hashes: vec![StreamTextHash {
                            stream_id: "raw_json".to_string(),
                            utf8_bytes: stream.raw_utf8_bytes,
                            sha256: stream.raw_sha256,
                        }],
                        final_usage: stream.final_usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some("局部补丁提案已保存".to_string()),
                        batch_summary: None,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return fail_optimize_patch_generation_execution(
                self,
                execution,
                request.session_id,
                Some(workflow.id),
                Some(pending.turn_id),
                generation_terminal_error(error),
            )
            .await;
        }
        execution.finish();
        self.finalize_workflow_run_outcome_validated(
            workflow.id,
            WorkflowRunFinalizeOutcome::Completed,
            None,
        )
        .await
        .map_err(|error| format!("局部补丁提案已保存，但 workflow 完成状态写入失败: {error}"))?;
        Ok(())
    }
}

fn patch_set_input(pending: &PendingPatchGeneration) -> OptimizationPatchSetCreateInput {
    OptimizationPatchSetCreateInput {
        turn_id: pending.turn_id,
        session_id: pending.session_id,
        chapter_id: pending.chapter_id,
        base_content_hash: pending.base_content_hash.clone(),
        status: "pending".to_string(),
        summary: pending.proposal.summary.clone(),
        patch_decision: "pending".to_string(),
        patch_apply_status: "pending".to_string(),
        patches: pending
            .proposal
            .patches
            .iter()
            .map(|patch| OptimizationPatchCreateInput {
                patch_id: patch.patch_id.clone(),
                issue_id: patch.issue_id.clone(),
                operation: patch.operation.clone(),
                old_text: patch.old_text.clone(),
                new_text: patch.new_text.clone(),
                reason: patch.reason.clone(),
                risk: patch.risk.clone(),
            })
            .collect(),
    }
}

async fn fail_optimize_patch_generation_execution(
    orchestrator: &OrchestratorContext,
    execution: GenerationTaskExecution,
    session_id: i64,
    workflow_run_id: Option<i64>,
    turn_id: Option<i64>,
    error: a4n_service_protocol::GenerationError,
) -> Result<(), String> {
    let cancelled = is_generation_cancelled(execution.session.snapshot().status, &error);
    if let Some(turn_id) = turn_id {
        let status = if cancelled { "cancelled" } else { "failed" };
        if let Err(turn_error) = orchestrator
            .fail_turn(turn_id, status, &error.message)
            .await
        {
            tracing::error!(turn_id, error = %turn_error, "局部补丁 Turn 失败状态写入失败");
        }
    }
    if let Err(event_error) = orchestrator
        .save_agent_event(session_id, &format!("局部补丁生成失败：{}", error.message))
        .await
    {
        tracing::error!(session_id, error = %event_error, "局部补丁失败事件写入失败");
    }
    let outcome = if cancelled {
        WorkflowRunFinalizeOutcome::Cancelled
    } else {
        WorkflowRunFinalizeOutcome::Failed
    };
    let workflow_error = if let Some(workflow_run_id) = workflow_run_id {
        orchestrator
            .finalize_workflow_run_outcome_validated(workflow_run_id, outcome, Some(&error.message))
            .await
            .err()
    } else {
        None
    };
    let mut message = execution.fail(error).await;
    if let Some(workflow_error) = workflow_error {
        message.push_str(&format!(
            "; 记录局部补丁 workflow 终态失败: {workflow_error}"
        ));
    }
    Err(message)
}
