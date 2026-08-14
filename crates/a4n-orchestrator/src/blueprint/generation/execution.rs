use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use a4n_api_contract::{
    blueprint::StartBlueprintGenerationInput, generation_tasks::GenerationTaskHandle,
};
use a4n_core::blueprint::{
    BLUEPRINT_GENERATE_PROGRESS_TOTAL, blueprint_stage_previous_context,
    build_blueprint_generation_payload,
};
use a4n_core::error::AppError;
use a4n_domain::progress::{ProgressPhaseKind, ProgressSeverity};
use a4n_service_protocol::{
    GenerationError, GenerationFinishReason, GenerationResultRef, ProgressEvent, StreamTextHash,
};
use futures::future::BoxFuture;
use serde_json::Value;

use crate::blueprint::generation::assembly::assemble_staged_blueprint_json;
use crate::blueprint::generation::completion::{
    BlueprintCompletionOutcome, BlueprintGenerationCompletionInput,
    persist_generated_blueprint_in_transaction,
};
use crate::blueprint::generation::stage::{BlueprintStageRunInput, ensure_not_cancelled};
use crate::blueprint::generation::trace::blueprint_stage_trace_requests;
use crate::blueprint::json::parse_blueprint_json;
use crate::blueprint::{BlueprintGenerationApplicationContext, OrchestratorContext};
use crate::generation::{
    events::GenerationEventPublisher, generation_public_error, terminal::CompletionDescriptor,
};
use crate::workflow::WorkflowRunFinalizeOutcome;

impl OrchestratorContext {
    /// 在唯一 generation worker/session/terminal owner 中执行蓝图生成。
    pub async fn run_blueprint_generation_task(
        &self,
        context: BlueprintGenerationApplicationContext<'_>,
        _user_id: i64,
        input: StartBlueprintGenerationInput,
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
        let project_id = input.project_id;
        let variables = input
            .variables
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect::<HashMap<_, _>>();
        let variables_json = match serde_json::to_value(&variables) {
            Ok(value) => value,
            Err(error) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    None,
                    generation_public_error(
                        "generation_request_invalid",
                        "blueprint_prepare",
                        error,
                    ),
                )
                .await;
            }
        };
        let workflow = match self
            .begin_blueprint_generation_workflow_validated(
                project_id,
                &input.title,
                &input.genre,
                input.target_length,
                &variables_json,
                Some(handle.task_id.clone()),
            )
            .await
        {
            Ok(workflow) => workflow,
            Err(error) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    None,
                    generation_public_error(
                        "generation_persist_failed",
                        "blueprint_workflow",
                        error,
                    ),
                )
                .await;
            }
        };
        let workflow_run_id = workflow.id;
        let expected_source_snapshot =
            match blueprint_generation_source_snapshot(self, project_id).await {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    return fail_blueprint_generation_execution(
                        self,
                        execution,
                        Some(workflow_run_id),
                        generation_public_error(
                            "generation_persist_failed",
                            "blueprint_prepare",
                            error,
                        ),
                    )
                    .await;
                }
            };

        if let Err(error) = publish_blueprint_progress(
            &publisher,
            "blueprint_prepare",
            ProgressPhaseKind::Prepare,
            0,
            "正在准备蓝图生成上下文",
        )
        .await
        {
            return fail_blueprint_generation_execution(
                self,
                execution,
                Some(workflow_run_id),
                error,
            )
            .await;
        }
        if let Err(error) = ensure_not_cancelled(&cancel_token) {
            return fail_blueprint_generation_execution(
                self,
                execution,
                Some(workflow_run_id),
                generation_public_error("generation_interrupted", "blueprint_prepare", error),
            )
            .await;
        }
        let prompt_content = match context
            .prompt_renderer
            .render_prompt("blueprint", &variables, Some(project_id))
            .await
        {
            Ok(prompt) => prompt,
            Err(error) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    generation_public_error(
                        "generation_request_invalid",
                        "blueprint_prompt",
                        error,
                    ),
                )
                .await;
            }
        };
        let user_payload = match build_blueprint_generation_payload(
            &input.title,
            &input.genre,
            input.target_length,
            &variables,
        ) {
            Ok(payload) => payload,
            Err(error) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    generation_public_error(
                        "generation_request_invalid",
                        "blueprint_prepare",
                        error,
                    ),
                )
                .await;
            }
        };
        let user_payload_json = match serde_json::to_string_pretty(&user_payload) {
            Ok(payload) => payload,
            Err(error) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    generation_public_error(
                        "generation_request_invalid",
                        "blueprint_prepare",
                        error,
                    ),
                )
                .await;
            }
        };

        let mut stage_outputs = Vec::with_capacity(BLUEPRINT_GENERATE_PROGRESS_TOTAL as usize);
        for (stage_index, spec) in crate::blueprint::BLUEPRINT_GENERATION_STAGES
            .iter()
            .enumerate()
        {
            if let Err(error) = publish_blueprint_progress(
                &publisher,
                spec.key,
                ProgressPhaseKind::Prepare,
                spec.progress_current,
                spec.progress_message,
            )
            .await
            {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    error,
                )
                .await;
            }
            let previous_context =
                match blueprint_stage_previous_context(stage_index, &stage_outputs) {
                    Ok(value) => value,
                    Err(error) => {
                        return fail_blueprint_generation_execution(
                            self,
                            execution,
                            Some(workflow_run_id),
                            generation_public_error(
                                "generation_validation_failed",
                                spec.key,
                                error,
                            ),
                        )
                        .await;
                    }
                };
            let value = match self
                .generate_or_resume_blueprint_stage(BlueprintStageRunInput {
                    project_id,
                    spec,
                    publisher: &publisher,
                    session: &session,
                    checkpoint_writer: &checkpoint_writer,
                    cancel_token: cancel_token.clone(),
                    prompt_renderer: context.prompt_renderer,
                    base_prompt: &prompt_content,
                    user_payload_json: &user_payload_json,
                    previous_context: previous_context.as_ref(),
                    workflow_run_id,
                })
                .await
            {
                Ok(value) => value,
                Err(error) => {
                    let code = if matches!(error, AppError::Cancelled) {
                        "generation_interrupted"
                    } else {
                        "generation_validation_failed"
                    };
                    return fail_blueprint_generation_execution(
                        self,
                        execution,
                        Some(workflow_run_id),
                        generation_public_error(code, spec.key, error),
                    )
                    .await;
                }
            };
            stage_outputs.push(value);
        }

        let [
            core_output,
            world_output,
            characters_output,
            relationships_output,
        ]: [Value; 4] = match stage_outputs.try_into() {
            Ok(outputs) => outputs,
            Err(_) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    generation_public_error(
                        "generation_validation_failed",
                        "blueprint_assemble",
                        "蓝图生成阶段输出数量不完整",
                    ),
                )
                .await;
            }
        };
        if let Err(error) = publish_blueprint_progress(
            &publisher,
            "blueprint_assemble",
            ProgressPhaseKind::Parse,
            6,
            "正在组合并校验完整蓝图结构",
        )
        .await
        {
            return fail_blueprint_generation_execution(
                self,
                execution,
                Some(workflow_run_id),
                error,
            )
            .await;
        }
        let assembled = assemble_staged_blueprint_json(
            &input.title,
            &input.genre,
            input.target_length,
            &core_output,
            &world_output,
            &characters_output,
            &relationships_output,
        );
        let assembled_content = match serde_json::to_string(&assembled) {
            Ok(value) => value,
            Err(error) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    generation_public_error(
                        "generation_validation_failed",
                        "blueprint_assemble",
                        error,
                    ),
                )
                .await;
            }
        };
        let parsed = match parse_blueprint_json(&assembled_content, input.target_length) {
            Ok(value) => value,
            Err(error) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    generation_public_error(
                        "generation_validation_failed",
                        "blueprint_validate",
                        error,
                    ),
                )
                .await;
            }
        };
        let trace_requests = match blueprint_stage_trace_requests(
            context.prompt_renderer,
            project_id,
            &prompt_content,
            &user_payload_json,
            &[
                core_output,
                world_output,
                characters_output,
                relationships_output,
            ],
        )
        .await
        {
            Ok(requests) => requests,
            Err(error) => {
                return fail_blueprint_generation_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    generation_public_error("generation_request_invalid", "blueprint_trace", error),
                )
                .await;
            }
        };

        let completion_slot: Arc<Mutex<Option<BlueprintCompletionOutcome>>> =
            Arc::new(Mutex::new(None));
        let completion_slot_for_tx = Arc::clone(&completion_slot);
        let completion_input = BlueprintGenerationCompletionInput {
            project_id,
            novel_title: input.title.clone(),
            genre: input.genre.clone(),
            target_length: input.target_length,
            user_payload,
            trace_requests,
            assembled_content,
            parsed,
            expected_source_snapshot,
            model_name: self.llm_model_name(),
        };
        let snapshot = session.snapshot();
        let raw_stream = snapshot
            .streams
            .iter()
            .find(|stream| stream.stream_id == "raw_json")
            .cloned();
        let final_text_hashes = raw_stream
            .as_ref()
            .map(|stream| StreamTextHash {
                stream_id: stream.stream_id.clone(),
                utf8_bytes: stream.text.len() as u64,
                sha256: stream.sha256.clone(),
            })
            .into_iter()
            .collect::<Vec<_>>();
        let final_usage = snapshot.usage.clone();
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                Box::pin(async move {
                    let outcome =
                        persist_generated_blueprint_in_transaction(tx, completion_input).await?;
                    let descriptor = CompletionDescriptor {
                        result_ref: Some(GenerationResultRef {
                            kind: "novel_blueprint".to_string(),
                            id: outcome.blueprint.id.to_string(),
                            revision: outcome.blueprint.current_version.clone(),
                        }),
                        final_text_hashes,
                        final_usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some("故事蓝图已生成".to_string()),
                        batch_summary: None,
                    };
                    *completion_slot_for_tx
                        .lock()
                        .map_err(|_| AppError::InvalidState {
                            current: "completion_slot_poisoned".to_string(),
                            operation: "保存蓝图生成结果".to_string(),
                        })? = Some(outcome);
                    Ok(descriptor)
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return fail_blueprint_generation_execution(
                self,
                execution,
                Some(workflow_run_id),
                generation_public_error(error.code(), "blueprint_persist", error),
            )
            .await;
        }
        execution.finish();
        let outcome = completion_slot
            .lock()
            .map_err(|_| "蓝图生成结果槽位损坏".to_string())?
            .take()
            .ok_or_else(|| "generation completed 但缺少蓝图结果".to_string())?;
        if let Some((from, to, timestamp)) = outcome.status_change {
            self.publish_internal_event(
                a4n_core::event::bridge::InternalEvent::ProjectStatusChanged {
                    project_id,
                    from,
                    to,
                    timestamp,
                },
            )
            .map_err(|error| format!("蓝图已保存，但项目状态事件发布失败: {error}"))?;
        }
        self.publish_internal_event(a4n_core::event::bridge::InternalEvent::BlueprintUpdated {
            project_id,
            version: outcome.blueprint.current_version.clone(),
        })
        .map_err(|error| format!("蓝图已保存，但蓝图更新事件发布失败: {error}"))?;
        self.finalize_workflow_run_outcome_validated(
            workflow_run_id,
            WorkflowRunFinalizeOutcome::Completed,
            None,
        )
        .await
        .map_err(|error| format!("蓝图已保存，但 workflow 完成状态写入失败: {error}"))?;
        Ok(())
    }
}

async fn blueprint_generation_source_snapshot(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Value, AppError> {
    let rows = a4n_db::outline::load_outline_generation_source_rows(
        context.database.connection(),
        project_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if rows.project.lifecycle_gate != a4n_db::generation::PROJECT_GATE_ACTIVE {
        return Err(AppError::InvalidState {
            current: rows.project.lifecycle_gate,
            operation: "生成故事蓝图".to_string(),
        });
    }
    Ok(a4n_db::outline::outline_generation_source_snapshot(&rows))
}

async fn publish_blueprint_progress(
    publisher: &dyn GenerationEventPublisher,
    stage_code: &str,
    phase_kind: ProgressPhaseKind,
    current: u32,
    message: &str,
) -> Result<(), GenerationError> {
    publisher
        .progress(ProgressEvent {
            stage_code: stage_code.to_string(),
            phase_kind,
            message: message.to_string(),
            current: Some(current as u64),
            total: Some(BLUEPRINT_GENERATE_PROGRESS_TOTAL as u64),
            unit_label: None,
            item_key: None,
            item_index: None,
            item_total: None,
            severity: ProgressSeverity::Info,
            estimated: false,
        })
        .await
        .map_err(|error| generation_public_error(error.code(), stage_code, error))
}

async fn fail_blueprint_generation_execution(
    orchestrator: &OrchestratorContext,
    execution: crate::context::GenerationTaskExecution,
    workflow_run_id: Option<i64>,
    error: GenerationError,
) -> Result<(), String> {
    let outcome = if execution.session.snapshot().status
        == a4n_service_protocol::GenerationTaskStatus::Cancelling
    {
        WorkflowRunFinalizeOutcome::Cancelled
    } else {
        WorkflowRunFinalizeOutcome::Failed
    };
    let workflow_error = if let Some(workflow_run_id) = workflow_run_id {
        let error_message = error.message.clone();
        orchestrator
            .finalize_workflow_run_outcome_validated(workflow_run_id, outcome, Some(&error_message))
            .await
            .err()
    } else {
        None
    };
    let mut message = execution.fail(error).await;
    if let Some(error) = workflow_error {
        message.push_str(&format!("; 记录蓝图 workflow 终态失败: {error}"));
    }
    Err(message)
}
