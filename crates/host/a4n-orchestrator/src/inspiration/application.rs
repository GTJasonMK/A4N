use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, inspiration::StartInspirationGenerationInput,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::ProgressPhaseKind;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::{GenerationFinishReason, GenerationResultRef, StreamTextHash};
use futures::future::BoxFuture;
use sha2::{Digest, Sha256};

use crate::{
    OrchestratorContext,
    context::fail_generation_workflow_execution,
    generation::{
        checkpoint::GenerationCheckpointMetadata,
        events::GenerationEventPublisher,
        generation_finish_reason_text, generation_progress as inspiration_progress,
        generation_public_error, generation_session_error, generation_terminal_error,
        stream::{
            LlmGenerationCheckpoint, LlmGenerationOutput, LlmGenerationRequest,
            execute_llm_generation,
        },
        terminal::CompletionDescriptor,
    },
};

use super::{
    request::build_inspiration_request_from_context, validation::validate_inspiration_response_json,
};

impl OrchestratorContext {
    pub async fn prepare_inspiration_request_validated(
        &self,
        project_id: i64,
        user_input: &str,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let (history_result, project_context_result) = futures::join!(
            self.inspiration_messages(project_id),
            self.inspiration_project_context_rows(project_id),
        );
        let history = history_result?;
        let project_context_rows = project_context_result?;
        build_inspiration_request_from_context(
            project_id,
            user_input,
            prompt_renderer,
            &history,
            project_context_rows,
        )
        .await
    }

    pub async fn run_inspiration_generation_task(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartInspirationGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        let Some(execution) = self
            .acquire_generation_task_execution(&handle)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let project_id = input.project_id;
        let user_input = input.message;
        if let Err(error) = self.ensure_inspiration_conversation(project_id).await {
            return fail_generation_workflow_execution(
                self,
                execution,
                None,
                generation_public_error(
                    "generation_persist_failed",
                    "inspiration_conversation",
                    error,
                ),
            )
            .await;
        }
        let session = execution.session.clone();
        let cancel_token = execution.permit.cancellation_token();
        let publisher = execution.publisher.clone();

        if let Err(error) = publisher
            .progress(inspiration_progress(
                "inspiration_prepare",
                ProgressPhaseKind::Prepare,
                "正在准备灵感对话上下文",
            ))
            .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                None,
                generation_session_error(error),
            )
            .await;
        }

        let request = match self
            .prepare_inspiration_request_validated(project_id, &user_input, prompt_renderer)
            .await
        {
            Ok(request) => request,
            Err(error) => {
                return fail_generation_workflow_execution(
                    self,
                    execution,
                    None,
                    generation_public_error(
                        "generation_request_invalid",
                        "inspiration_prepare",
                        error,
                    ),
                )
                .await;
            }
        };
        let workflow = match self
            .begin_inspiration_workflow_validated(
                project_id,
                "stream",
                &request,
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
                        "inspiration_workflow",
                        error,
                    ),
                )
                .await;
            }
        };
        let checkpoint_request = match self
            .prepare_inspiration_checkpoint_request(project_id, request, Some(workflow.id))
            .await
        {
            Ok(request) => request,
            Err(error) => {
                return fail_generation_workflow_execution(
                    self,
                    execution,
                    Some(workflow.id),
                    generation_public_error(
                        "generation_checkpoint_unavailable",
                        "checkpoint",
                        error,
                    ),
                )
                .await;
            }
        };
        let checkpoint_writer = execution.checkpoint_writer.clone();
        let checkpoint_metadata = GenerationCheckpointMetadata {
            step_key: Some("inspiration_conversation".to_string()),
            call_key: Some(checkpoint_request.call_key.clone()),
            provider_model: Some(self.llm_model_name()),
            ..Default::default()
        };
        let restored = match self
            .load_inspiration_checkpoint(
                project_id,
                checkpoint_request.workflow_run_id,
                &checkpoint_request.fingerprint_json,
            )
            .await
        {
            Ok(restored) => restored,
            Err(error) => {
                return fail_generation_workflow_execution(
                    self,
                    execution,
                    Some(workflow.id),
                    generation_public_error(
                        "generation_checkpoint_unavailable",
                        "checkpoint",
                        error,
                    ),
                )
                .await;
            }
        };
        let output = if let Some(content) = restored {
            if let Err(error) = publisher.text_delta("raw_json", &content).await {
                return fail_generation_workflow_execution(
                    self,
                    execution,
                    Some(workflow.id),
                    generation_session_error(error),
                )
                .await;
            }
            LlmGenerationOutput {
                content_sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
                content,
                usage: None,
                finish_reason: GenerationFinishReason::Stop,
            }
        } else {
            match execute_llm_generation(
                self.llm_backend(),
                &publisher,
                LlmGenerationRequest {
                    request: checkpoint_request.request,
                    stream_id: "raw_json".to_string(),
                    cancellation: cancel_token,
                    checkpoint: Some(LlmGenerationCheckpoint {
                        writer: &checkpoint_writer,
                        session: &session,
                        metadata: checkpoint_metadata.clone(),
                    }),
                    usage_base: None,
                },
            )
            .await
            {
                Ok(output) => output,
                Err(error) => {
                    return fail_generation_workflow_execution(
                        self,
                        execution,
                        Some(workflow.id),
                        error,
                    )
                    .await;
                }
            }
        };

        if let Err(error) = checkpoint_writer
            .persist_if_due(&session, checkpoint_metadata, true)
            .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow.id),
                generation_public_error("generation_checkpoint_unavailable", "checkpoint", error),
            )
            .await;
        }
        if let Err(error) = publisher
            .progress(inspiration_progress(
                "inspiration_parse",
                ProgressPhaseKind::Parse,
                "正在校验灵感对话响应",
            ))
            .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow.id),
                generation_session_error(error),
            )
            .await;
        }
        let validated = match validate_inspiration_response_json(&output.content) {
            Ok(validated) => validated,
            Err(error) => {
                return fail_generation_workflow_execution(
                    self,
                    execution,
                    Some(workflow.id),
                    generation_public_error(
                        "generation_validation_failed",
                        "inspiration_validate",
                        error,
                    ),
                )
                .await;
            }
        };
        if let Err(error) = self
            .save_inspiration_checkpoint(
                checkpoint_request.workflow_run_id,
                checkpoint_request.call_key,
                checkpoint_request.fingerprint_json,
                &validated,
                Some(generation_finish_reason_text(output.finish_reason)),
                output.usage.clone(),
            )
            .await
        {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow.id),
                generation_public_error(
                    "generation_persist_failed",
                    "inspiration_checkpoint",
                    error,
                ),
            )
            .await;
        }

        let raw_utf8_bytes = output.content.len() as u64;
        let raw_sha256 = output.content_sha256;
        let finish_reason = output.finish_reason;
        let final_usage = output.usage;
        let completion_owner = self.clone();
        let completion_user_input = user_input.clone();
        let completion_output = validated.clone();
        let completion = execution
            .terminal
            .complete_with(&session, move |tx| {
                let owner = completion_owner.clone();
                let user_input = completion_user_input.clone();
                let output = completion_output.clone();
                let raw_sha256 = raw_sha256.clone();
                let final_usage = final_usage.clone();
                Box::pin(async move {
                    let assistant_message_id = owner
                        .save_inspiration_exchange_in_transaction(
                            tx,
                            project_id,
                            &user_input,
                            &output,
                        )
                        .await?;
                    Ok(CompletionDescriptor {
                        result_ref: Some(GenerationResultRef {
                            kind: "inspiration_message".to_string(),
                            id: assistant_message_id.to_string(),
                            revision: format!("{:x}", Sha256::digest(output.as_bytes())),
                        }),
                        final_text_hashes: vec![StreamTextHash {
                            stream_id: "raw_json".to_string(),
                            utf8_bytes: raw_utf8_bytes,
                            sha256: raw_sha256,
                        }],
                        final_usage,
                        finish_reason: Some(finish_reason),
                        summary: Some("灵感对话回复已保存".to_string()),
                        batch_summary: None,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return fail_generation_workflow_execution(
                self,
                execution,
                Some(workflow.id),
                generation_terminal_error(error),
            )
            .await;
        }

        execution.finish();
        self.complete_workflow_run_validated(workflow.id)
            .await
            .map_err(|error| format!("灵感对话已保存，但 workflow 完成状态写入失败: {error}"))?;
        Ok(())
    }
}
