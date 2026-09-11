mod call;
mod contracts;
mod persistence;
mod preparation;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use a4n_api_contract::{
    blueprint::StartBlueprintRefinementInput, generation_tasks::GenerationTaskHandle,
};
use a4n_domain::progress::ProgressPhaseKind;
use a4n_domain::project::{BLUEPRINT_RAW_JSON_STREAM_ID, BLUEPRINT_REFINEMENT_WORKFLOW_KIND};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::{GenerationError, GenerationResultRef, StreamTextHash};
use futures::future::BoxFuture;

use crate::OrchestratorContext;
use crate::context::fail_generation_workflow_execution;
use crate::generation::{
    events::GenerationEventPublisher, generation_progress, generation_public_error,
    terminal::CompletionDescriptor,
};

use self::call::execute_blueprint_refine_call;
use self::persistence::persist_blueprint_refinement_in_transaction;
use self::preparation::prepare_blueprint_refine;

impl OrchestratorContext {
    pub async fn run_blueprint_refinement_task(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartBlueprintRefinementInput,
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
        let StartBlueprintRefinementInput {
            meta: _,
            project_id,
            focus,
            variables,
        } = input;
        let variables = variables
            .unwrap_or_default()
            .into_iter()
            .collect::<HashMap<_, _>>();
        if let Err(error) = publish_refinement_progress(
            &publisher,
            "blueprint_refine_prepare",
            ProgressPhaseKind::Prepare,
            "正在准备蓝图优化上下文",
        )
        .await
        {
            return fail_generation_workflow_execution(self, execution, None, error).await;
        }
        let prepared = match prepare_blueprint_refine(
            self,
            project_id,
            &handle.task_id,
            focus.as_deref(),
            prompt_renderer,
            &variables,
        )
        .await
        {
            Ok(prepared) => prepared,
            Err(error) => {
                return fail_generation_workflow_execution(
                    self,
                    execution,
                    None,
                    generation_public_error(
                        "generation_request_invalid",
                        "blueprint_refine_prepare",
                        error,
                    ),
                )
                .await;
            }
        };
        let workflow_run_id = prepared.workflow_run_id;
        if let Err(error) = publish_refinement_progress(
            &publisher,
            "blueprint_refine_stream",
            ProgressPhaseKind::ModelStream,
            "正在流式生成蓝图优化结果",
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
        let output = match execute_blueprint_refine_call(
            self,
            &prepared,
            &publisher,
            &session,
            &checkpoint_writer,
            cancel_token.clone(),
        )
        .await
        {
            Ok(output) => output,
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
                    generation_public_error(code, "blueprint_refine_call", error),
                )
                .await;
            }
        };
        if let Err(error) = checkpoint_writer
            .persist_if_due(
                &session,
                crate::generation::checkpoint::GenerationCheckpointMetadata {
                    step_key: Some(BLUEPRINT_REFINEMENT_WORKFLOW_KIND.to_string()),
                    call_key: Some(prepared.call_key.clone()),
                    provider_model: Some(self.llm_model_name()),
                    ..Default::default()
                },
                true,
            )
            .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                generation_public_error("generation_checkpoint_unavailable", "checkpoint", error),
            )
            .await;
        }
        if let Err(error) = publish_refinement_progress(
            &publisher,
            "blueprint_refine_validate",
            ProgressPhaseKind::Validate,
            "正在校验蓝图优化结果",
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

        let completion_slot: Arc<
            Mutex<Option<a4n_db_models::models::project::novel_blueprint::Model>>,
        > = Arc::new(Mutex::new(None));
        let completion_slot_for_tx = Arc::clone(&completion_slot);
        let expected_source_snapshot = prepared.expected_source_snapshot.clone();
        let parsed = output.parsed;
        let snapshot = session.snapshot();
        let final_text_hashes = snapshot
            .streams
            .iter()
            .find(|stream| stream.stream_id == BLUEPRINT_RAW_JSON_STREAM_ID)
            .map(|stream| StreamTextHash {
                stream_id: stream.stream_id.clone(),
                utf8_bytes: stream.text.len() as u64,
                sha256: stream.sha256.clone(),
            })
            .into_iter()
            .collect::<Vec<_>>();
        let final_usage = output.usage.or(snapshot.usage.clone());
        let finish_reason = output.finish_reason;
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                Box::pin(async move {
                    let blueprint = persist_blueprint_refinement_in_transaction(
                        tx,
                        project_id,
                        expected_source_snapshot,
                        parsed,
                    )
                    .await?;
                    let descriptor = CompletionDescriptor {
                        result_ref: Some(GenerationResultRef {
                            kind: "novel_blueprint".to_string(),
                            id: blueprint.id.to_string(),
                            revision: blueprint.current_version.clone(),
                        }),
                        final_text_hashes,
                        final_usage,
                        finish_reason: Some(finish_reason),
                        summary: Some("蓝图优化已保存".to_string()),
                        batch_summary: None,
                    };
                    *completion_slot_for_tx
                        .lock()
                        .map_err(|_| AppError::InvalidState {
                            current: "completion_slot_poisoned".to_string(),
                            operation: "保存蓝图优化结果".to_string(),
                        })? = Some(blueprint);
                    Ok(descriptor)
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow_run_id),
                generation_public_error(error.code(), "blueprint_refine_persist", error),
            )
            .await;
        }
        execution.finish();
        let blueprint = completion_slot
            .lock()
            .map_err(|_| "蓝图优化结果槽位损坏".to_string())?
            .take()
            .ok_or_else(|| "generation completed 但缺少蓝图优化结果".to_string())?;
        self.publish_internal_event(a4n_event::InternalEvent::BlueprintUpdated {
            project_id,
            version: blueprint.current_version.clone(),
        })
        .map_err(|error| format!("蓝图优化已保存，但蓝图更新事件发布失败: {error}"))?;
        self.complete_workflow_run_validated(workflow_run_id)
            .await
            .map_err(|error| format!("蓝图优化已保存，但 workflow 完成状态写入失败: {error}"))?;
        Ok(())
    }
}

async fn publish_refinement_progress(
    publisher: &dyn GenerationEventPublisher,
    stage_code: &str,
    phase_kind: ProgressPhaseKind,
    message: &str,
) -> Result<(), GenerationError> {
    publisher
        .progress(generation_progress(stage_code, phase_kind, message))
        .await
        .map_err(|error| generation_public_error(error.code(), stage_code, error))
}
