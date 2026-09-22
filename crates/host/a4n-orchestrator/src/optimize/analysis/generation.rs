use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, optimize::StartOptimizeAnalysisGenerationInput,
};
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use a4n_service_protocol::{
    GenerationFinishReason, GenerationResultRef, GenerationTaskStatus, StreamTextHash,
};
use futures::future::BoxFuture;
use sha2::{Digest, Sha256};

use crate::{
    OrchestratorContext,
    context::{GenerationTaskExecution, fail_generation_workflow_execution},
    generation::{
        checkpoint::GenerationCheckpointMetadata, events::GenerationEventPublisher,
        generation_failure_is_cancelled_or_interrupted, generation_session_error,
        generation_terminal_error,
    },
    optimize::{
        AgentLoopGenerationContext, agent_loop_error_to_generation_error,
        app_error_to_generation_error, generation_stream_completion, optimize_db_error,
        optimize_progress,
    },
};

impl OrchestratorContext {
    pub async fn run_optimize_analysis_generation_task(
        &self,
        user_id: i64,
        input: StartOptimizeAnalysisGenerationInput,
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

        let optimize_session = match self
            .optimize_session_for_user(user_id, input.session_id)
            .await
        {
            Ok(session) => session,
            Err(error) => {
                return fail_optimize_analysis_generation_execution(
                    self,
                    execution,
                    input.session_id,
                    None,
                    None,
                    app_error_to_generation_error(error, "optimization_analysis"),
                )
                .await;
            }
        };
        if let Err(error) = publisher
            .progress(optimize_progress(
                "optimization_analysis_prepare",
                ProgressPhaseKind::Prepare,
                "正在准备内容优化分析",
            ))
            .await
        {
            return fail_optimize_analysis_generation_execution(
                self,
                execution,
                input.session_id,
                None,
                None,
                generation_session_error(error),
            )
            .await;
        }

        let workflow = match self
            .begin_optimize_session_workflow_validated(
                optimize_session.id,
                "content_optimization_analysis",
                "analysis",
                optimize_session.project_id,
                optimize_session.chapter_id,
                Some(handle.task_id.clone()),
            )
            .await
        {
            Ok(workflow) => workflow,
            Err(error) => {
                return fail_optimize_analysis_generation_execution(
                    self,
                    execution,
                    input.session_id,
                    None,
                    None,
                    app_error_to_generation_error(error, "optimization_analysis_workflow"),
                )
                .await;
            }
        };

        let generation = AgentLoopGenerationContext {
            publisher: &publisher,
            cancellation: &cancel_token,
            checkpoint_writer: &checkpoint_writer,
            session: &session,
            agent_max_output_tokens: None,
        };
        let pending = match self
            .run_analysis_session(
                input.session_id,
                workflow.id,
                input.meta.retry_of_task_id.is_some(),
                &generation,
            )
            .await
        {
            Ok(pending) => pending,
            Err(error) => {
                return fail_optimize_analysis_generation_execution(
                    self,
                    execution,
                    input.session_id,
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
            return fail_optimize_analysis_generation_execution(
                self,
                execution,
                input.session_id,
                Some(workflow.id),
                Some(pending.turn_id),
                crate::generation::generation_public_error(
                    "generation_interrupted",
                    "provider_stream",
                    "内容优化分析已取消",
                ),
            )
            .await;
        }

        if let Err(error) = checkpoint_writer
            .persist_if_due(
                &session,
                GenerationCheckpointMetadata {
                    step_key: Some("optimization_analysis".to_string()),
                    provider_model: Some(self.llm_model_name()),
                    ..Default::default()
                },
                true,
            )
            .await
        {
            return fail_optimize_analysis_generation_execution(
                self,
                execution,
                input.session_id,
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
                return fail_optimize_analysis_generation_execution(
                    self,
                    execution,
                    input.session_id,
                    Some(workflow.id),
                    Some(pending.turn_id),
                    error,
                )
                .await;
            }
        };
        let session_id = input.session_id;
        let turn_id = pending.turn_id;
        let assistant_content = pending.assistant_response.clone();
        let canonical_issues = pending.canonical_issues.clone();
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                let assistant_content = assistant_content.clone();
                let canonical_issues = canonical_issues.clone();
                let stream = stream;
                Box::pin(async move {
                    let turn = a4n_db::optimize::complete_optimization_analysis_turn(
                        tx,
                        session_id,
                        turn_id,
                        &assistant_content,
                        canonical_issues,
                    )
                    .await
                    .map_err(optimize_db_error)?;
                    Ok(crate::generation::terminal::CompletionDescriptor {
                        result_ref: Some(GenerationResultRef {
                            kind: "optimization_turn".to_string(),
                            id: turn.id.to_string(),
                            revision: format!("{:x}", Sha256::digest(assistant_content.as_bytes())),
                        }),
                        final_text_hashes: vec![StreamTextHash {
                            stream_id: "raw_json".to_string(),
                            utf8_bytes: stream.raw_utf8_bytes,
                            sha256: stream.raw_sha256,
                        }],
                        final_usage: stream.final_usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some("内容优化分析已保存".to_string()),
                        batch_summary: None,
                    })
                })
                    as BoxFuture<
                        '_,
                        Result<crate::generation::terminal::CompletionDescriptor, AppError>,
                    >
            })
            .await;
        if let Err(error) = completion {
            return fail_optimize_analysis_generation_execution(
                self,
                execution,
                input.session_id,
                Some(workflow.id),
                Some(turn_id),
                generation_terminal_error(error),
            )
            .await;
        }

        execution.finish();
        self.complete_workflow_run_validated(workflow.id)
            .await
            .map_err(|error| {
                format!("内容优化分析已保存，但 workflow 完成状态写入失败: {error}")
            })?;
        Ok(())
    }
}

async fn fail_optimize_analysis_generation_execution(
    orchestrator: &OrchestratorContext,
    execution: GenerationTaskExecution,
    session_id: i64,
    workflow_run_id: Option<i64>,
    turn_id: Option<i64>,
    error: a4n_service_protocol::GenerationError,
) -> Result<(), String> {
    let cancelled =
        generation_failure_is_cancelled_or_interrupted(execution.session.snapshot().status, &error);
    if let Some(turn_id) = turn_id {
        let status = if cancelled { "cancelled" } else { "failed" };
        if let Err(turn_error) = orchestrator
            .fail_turn(turn_id, status, &error.message)
            .await
        {
            tracing::error!(turn_id, error = %turn_error, "内容优化分析 Turn 失败状态写入失败");
        }
    }
    let cleanup = orchestrator
        .record_command_error(session_id, "内容优化分析", &error.message, cancelled)
        .await
        .err();
    fail_generation_workflow_execution(orchestrator, execution, workflow_run_id, error)
        .await
        .map_err(|mut message| {
            if let Some(cleanup) = cleanup {
                message.push_str(&format!("; 记录内容优化分析失败状态失败: {cleanup}"));
            }
            message
        })
}
