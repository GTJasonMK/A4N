use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, optimize::StartOptimizeChatGenerationInput,
};
use a4n_core::error::AppError;
use a4n_domain::progress::ProgressPhaseKind;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::{
    GenerationError, GenerationFinishReason, GenerationResultRef, GenerationTaskStatus,
    StreamTextHash,
};
use futures::future::BoxFuture;
use sha2::{Digest, Sha256};

use crate::{
    OrchestratorContext,
    context::GenerationTaskExecution,
    generation::{
        checkpoint::GenerationCheckpointMetadata, events::GenerationEventPublisher,
        generation_public_error, generation_session_error, generation_terminal_error,
        terminal::CompletionDescriptor,
    },
    optimize::{
        AgentLoopGenerationContext, OptimizeAgentPermissions, OptimizeSendUserMessageInput,
        agent_loop_error_to_generation_error, app_error_to_generation_error, optimize_progress,
    },
    workflow::WorkflowRunFinalizeOutcome,
};

pub struct OptimizeChatGenerationApplicationContext<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
}

impl OrchestratorContext {
    pub async fn run_optimize_chat_generation_task(
        &self,
        context: OptimizeChatGenerationApplicationContext<'_>,
        user_id: i64,
        input: StartOptimizeChatGenerationInput,
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
        let checkpoint_metadata = GenerationCheckpointMetadata {
            step_key: Some("optimization_chat".to_string()),
            provider_model: Some(self.llm_model_name()),
            ..Default::default()
        };
        let request = input.request;

        let optimize_session = match self
            .validated_optimize_chat_generation_session(
                user_id,
                request.session_id,
                &request.message,
            )
            .await
        {
            Ok(session) => session,
            Err(error) => {
                return fail_optimize_generation_execution(
                    self,
                    execution,
                    None,
                    None,
                    app_error_to_generation_error(error, "optimize_chat"),
                )
                .await;
            }
        };

        if let Err(error) = publisher
            .progress(optimize_progress(
                "optimization_chat_prepare",
                ProgressPhaseKind::Prepare,
                "正在准备内容优化 Agent 对话",
            ))
            .await
        {
            return fail_optimize_generation_execution(
                self,
                execution,
                None,
                None,
                generation_session_error(error),
            )
            .await;
        }

        let workflow = match self
            .begin_optimize_session_workflow_validated(
                optimize_session.id,
                "content_optimization_chat",
                "chat",
                optimize_session.project_id,
                optimize_session.chapter_id,
                Some(handle.task_id.clone()),
            )
            .await
        {
            Ok(workflow) => workflow,
            Err(error) => {
                return fail_optimize_generation_execution(
                    self,
                    execution,
                    None,
                    None,
                    app_error_to_generation_error(error, "optimization_chat_workflow"),
                )
                .await;
            }
        };

        let permissions = request
            .permissions
            .as_ref()
            .map(OptimizeAgentPermissions::from);
        let generation = AgentLoopGenerationContext {
            publisher: &publisher,
            cancellation: &cancel_token,
            checkpoint_writer: &checkpoint_writer,
            session: &session,
        };
        let pending = match self
            .send_user_message_with_permissions(OptimizeSendUserMessageInput {
                session_id: request.session_id,
                message: &request.message,
                permissions_override: permissions.as_ref(),
                prompt_renderer: context.prompt_renderer,
                workflow_run_id: workflow.id,
                generation: &generation,
            })
            .await
        {
            Ok(pending) => pending,
            Err(error) => {
                return fail_optimize_generation_execution(
                    self,
                    execution,
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
            return fail_optimize_generation_execution(
                self,
                execution,
                Some(workflow.id),
                Some(pending.turn_id),
                generation_public_error(
                    "generation_interrupted",
                    "provider_stream",
                    "内容优化对话已取消",
                ),
            )
            .await;
        }

        if let Err(error) = checkpoint_writer
            .persist_if_due(&session, checkpoint_metadata.clone(), true)
            .await
        {
            return fail_optimize_generation_execution(
                self,
                execution,
                Some(workflow.id),
                Some(pending.turn_id),
                generation_public_error("generation_checkpoint_unavailable", "checkpoint", error),
            )
            .await;
        }

        let stream = match session
            .snapshot()
            .streams
            .into_iter()
            .find(|stream| stream.stream_id == "raw_json")
        {
            Some(stream) => stream,
            None => {
                return fail_optimize_generation_execution(
                    self,
                    execution,
                    Some(workflow.id),
                    Some(pending.turn_id),
                    generation_public_error(
                        "generation_checkpoint_unavailable",
                        "checkpoint",
                        "内容优化对话缺少 raw_json generation stream",
                    ),
                )
                .await;
            }
        };
        let raw_utf8_bytes = u64::try_from(stream.text.len())
            .map_err(|_| "内容优化对话 raw_json 长度超出 generation 协议范围".to_string());
        let raw_utf8_bytes = match raw_utf8_bytes {
            Ok(value) => value,
            Err(error) => {
                return fail_optimize_generation_execution(
                    self,
                    execution,
                    Some(workflow.id),
                    Some(pending.turn_id),
                    generation_public_error("generation_event_invalid", "checkpoint", error),
                )
                .await;
            }
        };
        let completion_user_text = pending.user_text.clone();
        let completion_reply = pending.final_reply.clone();
        let completion_stream_hash = stream.sha256;
        let completion_usage = session.snapshot().usage;
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                let user_text = completion_user_text.clone();
                let reply = completion_reply.clone();
                let stream_hash = completion_stream_hash.clone();
                let usage = completion_usage.clone();
                Box::pin(async move {
                    let turn = a4n_db::optimize::complete_optimization_chat_turn(
                        tx,
                        request.session_id,
                        pending.turn_id,
                        &user_text,
                        &reply,
                    )
                    .await
                    .map_err(crate::optimize::optimize_db_error)?;
                    Ok(CompletionDescriptor {
                        result_ref: Some(GenerationResultRef {
                            kind: "optimization_turn".to_string(),
                            id: turn.id.to_string(),
                            revision: format!("{:x}", Sha256::digest(reply.as_bytes())),
                        }),
                        final_text_hashes: vec![StreamTextHash {
                            stream_id: "raw_json".to_string(),
                            utf8_bytes: raw_utf8_bytes,
                            sha256: stream_hash,
                        }],
                        final_usage: usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some("内容优化 Agent 对话已保存".to_string()),
                        batch_summary: None,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return fail_optimize_generation_execution(
                self,
                execution,
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
        .map_err(|error| format!("内容优化对话已保存，但 workflow 完成状态写入失败: {error}"))?;
        Ok(())
    }
}

async fn fail_optimize_generation_execution(
    orchestrator: &OrchestratorContext,
    execution: GenerationTaskExecution,
    workflow_run_id: Option<i64>,
    turn_id: Option<i64>,
    error: GenerationError,
) -> Result<(), String> {
    if let Some(turn_id) = turn_id {
        let status = if execution.session.snapshot().status == GenerationTaskStatus::Cancelling
            || error.code.as_str() == "generation_interrupted"
        {
            "cancelled"
        } else {
            "failed"
        };
        if let Err(turn_error) = orchestrator
            .fail_turn(turn_id, status, &error.message)
            .await
        {
            tracing::error!(turn_id, error = %turn_error, "内容优化 Turn 失败状态写入失败");
        }
    }
    let outcome = if execution.session.snapshot().status == GenerationTaskStatus::Cancelling
        || error.code.as_str() == "generation_interrupted"
    {
        WorkflowRunFinalizeOutcome::Cancelled
    } else {
        WorkflowRunFinalizeOutcome::Failed
    };
    let error_message = error.message.clone();
    let workflow_error = if let Some(workflow_run_id) = workflow_run_id {
        orchestrator
            .finalize_workflow_run_outcome_validated(workflow_run_id, outcome, Some(&error_message))
            .await
            .err()
    } else {
        None
    };
    let mut message = execution.fail(error).await;
    if let Some(workflow_error) = workflow_error {
        message.push_str(&format!(
            "; 记录内容优化 workflow 终态失败: {workflow_error}"
        ));
    }
    Err(message)
}
