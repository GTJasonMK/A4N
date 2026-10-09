use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use a4n_api_contract::{
    blueprint::StartBlueprintGenerationInput, generation_tasks::GenerationTaskHandle,
};
use a4n_domain::project::{
    BLUEPRINT_BASE_PROMPT, BLUEPRINT_GENERATE_PROGRESS_TOTAL, BLUEPRINT_RAW_JSON_STREAM_ID,
    CHAPTERS_PER_PART, build_blueprint_generation_payload, needs_part_outlines_for_chapters,
};
use a4n_error::AppError;
use a4n_progress::{ProgressPhaseKind, ProgressSeverity};
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::{
    GenerationError, GenerationFinishReason, GenerationResultRef, ProgressEvent, StreamTextHash,
};
use futures::future::BoxFuture;
use serde_json::{Map, Value};

use crate::blueprint::OrchestratorContext;
use crate::blueprint::generation::completion::{
    BlueprintCompletionOutcome, BlueprintGenerationCompletionInput,
    persist_generated_blueprint_in_transaction,
};
use crate::blueprint::generation::stage::{BlueprintStageRunInput, ensure_not_cancelled};
use crate::blueprint::json::parse_blueprint_json;
use crate::context::fail_generation_workflow_execution;
use crate::generation::{
    events::GenerationEventPublisher, generation_public_error, terminal::CompletionDescriptor,
};

impl OrchestratorContext {
    /// 在唯一 generation worker/session/terminal owner 中执行蓝图生成。
    pub async fn run_blueprint_generation_task(
        &self,
        prompt_renderer: &dyn PromptRenderer,
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
        let StartBlueprintGenerationInput {
            meta: _,
            project_id,
            title,
            genre,
            target_length,
            variables,
        } = input;
        let variables = variables
            .unwrap_or_default()
            .into_iter()
            .collect::<HashMap<_, _>>();
        let variables_json = match serde_json::to_value(&variables) {
            Ok(value) => value,
            Err(error) => {
                return fail_generation_workflow_execution(
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
                &title,
                &genre,
                target_length,
                &variables_json,
                Some(handle.task_id.clone()),
            )
            .await
        {
            Ok(workflow) => workflow,
            Err(error) => {
                return fail_generation_workflow_execution(
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
                    return fail_generation_workflow_execution(
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
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                error,
            )
            .await;
        }
        if let Err(error) = ensure_not_cancelled(&cancel_token) {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                generation_public_error("generation_interrupted", "blueprint_prepare", error),
            )
            .await;
        }
        let prompt_content = match prompt_renderer
            .render_prompt_for_call(
                "OrchestratorContext.run_blueprint_generation_task",
                BLUEPRINT_BASE_PROMPT,
                &variables,
                Some(project_id),
            )
            .await
        {
            Ok(prompt) => prompt,
            Err(error) => {
                return fail_generation_workflow_execution(
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
        let user_payload =
            match build_blueprint_generation_payload(&title, &genre, target_length, &variables)
                .map_err(AppError::InvalidArgument)
            {
                Ok(payload) => payload,
                Err(error) => {
                    return fail_generation_workflow_execution(
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
                return fail_generation_workflow_execution(
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

        if let Err(error) = publish_blueprint_progress(
            &publisher,
            "blueprint_prompt",
            ProgressPhaseKind::Prepare,
            1,
            "已整理灵感对话与蓝图提示词",
        )
        .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                error,
            )
            .await;
        }

        let mut previous_stage_outputs = Map::new();
        let mut assembled = Map::new();
        let mut trace_requests =
            Vec::with_capacity(crate::blueprint::BLUEPRINT_GENERATION_STAGES.len());
        for spec in &crate::blueprint::BLUEPRINT_GENERATION_STAGES {
            if let Err(error) = publish_blueprint_progress(
                &publisher,
                spec.key,
                ProgressPhaseKind::Prepare,
                u64::from(spec.progress_current),
                spec.progress_message,
            )
            .await
            {
                return fail_generation_workflow_execution(
                    self,
                    execution,
                    Some(workflow_run_id),
                    error,
                )
                .await;
            }
            let previous_context = (!previous_stage_outputs.is_empty())
                .then(|| Value::Object(previous_stage_outputs.clone()));
            let stage = match self
                .generate_or_resume_blueprint_stage(BlueprintStageRunInput {
                    project_id,
                    spec,
                    publisher: &publisher,
                    session: &session,
                    checkpoint_writer: &checkpoint_writer,
                    cancel_token: cancel_token.clone(),
                    prompt_renderer,
                    base_prompt: &prompt_content,
                    user_payload_json: &user_payload_json,
                    previous_context: previous_context.as_ref(),
                    workflow_run_id,
                })
                .await
            {
                Ok(stage) => stage,
                Err(error) => {
                    let code = if matches!(error, AppError::Cancelled) {
                        "generation_interrupted"
                    } else {
                        "generation_validation_failed"
                    };
                    return fail_generation_workflow_execution(
                        self,
                        execution,
                        Some(workflow_run_id),
                        generation_public_error(code, spec.key, error),
                    )
                    .await;
                }
            };
            assembled.extend(
                stage
                    .output
                    .as_object()
                    .expect("validated blueprint stage output must be an object")
                    .clone(),
            );
            previous_stage_outputs.insert(spec.key.to_string(), stage.output);
            trace_requests.push((spec.key, stage.request));
        }
        if let Err(error) = publish_blueprint_progress(
            &publisher,
            "blueprint_assemble",
            ProgressPhaseKind::Parse,
            6,
            "正在组合并校验完整蓝图结构",
        )
        .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                error,
            )
            .await;
        }
        assembled.insert("chapter_outline".to_string(), Value::Array(Vec::new()));
        assembled.insert(
            "needs_part_outlines".to_string(),
            Value::Bool(needs_part_outlines_for_chapters(target_length)),
        );
        assembled.insert("total_chapters".to_string(), Value::from(target_length));
        assembled.insert(
            "chapters_per_part".to_string(),
            Value::from(CHAPTERS_PER_PART),
        );
        let assembled = Value::Object(assembled);
        let assembled_content = match serde_json::to_string(&assembled) {
            Ok(value) => value,
            Err(error) => {
                return fail_generation_workflow_execution(
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
        let parsed = match parse_blueprint_json(&assembled_content, target_length) {
            Ok(value) => value,
            Err(error) => {
                return fail_generation_workflow_execution(
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
        if let Err(error) = publish_blueprint_progress(
            &publisher,
            "blueprint_validate",
            ProgressPhaseKind::Validate,
            7,
            "正在校验章节规划与工作流边界",
        )
        .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                error,
            )
            .await;
        }
        if let Err(error) = publish_blueprint_progress(
            &publisher,
            "blueprint_preserve",
            ProgressPhaseKind::Prepare,
            8,
            "正在保留已有大纲与章节产物",
        )
        .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                error,
            )
            .await;
        }
        if let Err(error) = ensure_not_cancelled(&cancel_token) {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                generation_public_error("generation_interrupted", "blueprint_preserve", error),
            )
            .await;
        }
        let completion_slot: Arc<Mutex<Option<BlueprintCompletionOutcome>>> =
            Arc::new(Mutex::new(None));
        let completion_slot_for_tx = Arc::clone(&completion_slot);
        let completion_input = BlueprintGenerationCompletionInput {
            project_id,
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
            .find(|stream| stream.stream_id == BLUEPRINT_RAW_JSON_STREAM_ID)
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
            .complete_with_progress(&session, move |tx| {
                Box::pin(async move {
                    let (outcome, progress_events) =
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
                    Ok((descriptor, progress_events))
                })
                    as BoxFuture<
                        '_,
                        Result<
                            (
                                CompletionDescriptor,
                                Vec<a4n_service_protocol::ProgressEvent>,
                            ),
                            AppError,
                        >,
                    >
            })
            .await;
        if let Err(error) = completion {
            return fail_generation_workflow_execution(
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
            self.publish_internal_event(a4n_event::InternalEvent::ProjectStatusChanged {
                project_id,
                from,
                to,
                timestamp,
            })
            .map_err(|error| format!("蓝图已保存，但项目状态事件发布失败: {error}"))?;
        }
        self.publish_internal_event(a4n_event::InternalEvent::BlueprintUpdated {
            project_id,
            version: outcome.blueprint.current_version.clone(),
        })
        .map_err(|error| format!("蓝图已保存，但蓝图更新事件发布失败: {error}"))?;
        self.complete_workflow_run_validated(workflow_run_id)
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
        context.database.executor(),
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
    current: u64,
    message: &str,
) -> Result<(), GenerationError> {
    publisher
        .progress(ProgressEvent {
            stage_code: stage_code.to_string(),
            phase_kind,
            message: message.to_string(),
            current: Some(current),
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
