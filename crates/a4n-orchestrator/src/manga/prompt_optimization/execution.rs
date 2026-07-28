use a4n_core::{error::AppError, models::chapter::chapter_manga_prompt};
use a4n_llm::{
    provider::{ChatRequest, is_truncated_finish_reason},
    trace::context_aware_chat_request_trace_json,
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    manga::{
        MANGA_PAGE_PROMPT_TYPE, ensure_manga_source_can_write,
        prompt_optimization_workflow_rules::manga_prompt_optimization_workflow_run_input,
    },
    project::required_text,
    structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput},
    workflow::{self, CompletedLlmCallCheckpointInput, workflow_checkpoint_contract_app_error},
};

use super::{
    checkpoint::{
        OptimizedMangaPromptFields, ValidatedMangaPromptOptimization,
        restored_manga_prompt_optimization_checkpoint, validate_manga_prompt_optimization,
    },
    contracts::{
        MangaPromptDownstreamCleanupPort, MangaPromptOptimizationInput,
        MangaPromptOptimizationResult, PreparedMangaPromptOptimization,
        RequiredMangaPromptOptimizationInput,
    },
    request::{
        build_manga_prompt_optimization_request, manga_prompt_optimization_call_key,
        manga_prompt_optimization_input_fingerprint,
    },
};

impl OrchestratorContext {
    pub async fn optimize_manga_prompt_validated(
        &self,
        input: MangaPromptOptimizationInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup_port: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        let prepared = self
            .prepare_manga_prompt_optimization(input, prompt_renderer)
            .await?;
        let workflow_input = manga_prompt_optimization_workflow_run_input(
            prepared.panel.project_id,
            prepared.panel.id,
            prepared.panel.chapter_id,
            &prepared.panel.prompt_type,
            &prepared.call_key,
            prepared.input_fingerprint_json.clone(),
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = self.begin_workflow_run_validated(workflow_input).await?;
        let result = self
            .execute_manga_prompt_optimization(workflow_run.id, prepared, cleanup_port)
            .await;
        self.finalize_manga_prompt_optimization(workflow_run.id, result)
            .await
    }

    async fn prepare_manga_prompt_optimization(
        &self,
        input: MangaPromptOptimizationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<PreparedMangaPromptOptimization, AppError> {
        let required = required_manga_prompt_optimization_input(input)?;
        let source = self
            .chapter_manga_source_by_id(required.panel.chapter_id, required.panel.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "优化整页漫画提示词")?;
        let request = build_manga_prompt_optimization_request(
            prompt_renderer,
            &required.panel,
            &required.instruction,
        )
        .await?;
        let request_json = context_aware_chat_request_trace_json(&request)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("LLM 请求序列化失败: {err}")))?;
        let input_fingerprint_json = manga_prompt_optimization_input_fingerprint(
            &required.panel,
            &required.instruction,
            request_json,
        );
        let call_key = manga_prompt_optimization_call_key(required.panel.id);
        Ok(PreparedMangaPromptOptimization {
            panel: required.panel,
            downstream_image_ids: required.downstream_image_ids,
            call_key,
            input_fingerprint_json,
            request,
        })
    }

    async fn execute_manga_prompt_optimization(
        &self,
        workflow_run_id: i64,
        mut prepared: PreparedMangaPromptOptimization,
        cleanup_port: &dyn MangaPromptDownstreamCleanupPort,
    ) -> MangaPromptOptimizationResult {
        let input_fingerprint_sha256 =
            workflow::llm_call_checkpoint_fingerprint_sha256(&prepared.input_fingerprint_json)
                .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(trace_context) = prepared.request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(prepared.call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        self.mark_workflow_current_step(workflow_run_id, &prepared.call_key)
            .await?;
        let image_prompt = self
            .run_manga_prompt_optimization_call(
                prepared.request,
                workflow_run_id,
                &prepared.call_key,
                &prepared.input_fingerprint_json,
            )
            .await?;
        if prepared.panel.image_prompt.trim() == image_prompt {
            return Ok(prepared.panel);
        }
        let updated = self
            .update_manga_page_prompt_image_prompt(prepared.panel, image_prompt)
            .await?;
        if !prepared.downstream_image_ids.is_empty() {
            cleanup_port
                .delete_downstream_images(&prepared.downstream_image_ids)
                .await?;
        }
        Ok(updated)
    }

    async fn run_manga_prompt_optimization_call(
        &self,
        request: ChatRequest,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &Value,
    ) -> Result<String, AppError> {
        let (validated, checkpoint_response_text, response_usage_json, response_finish_reason) =
            if let Some(checkpoint) = self
                .load_completed_llm_call_checkpoint(
                    workflow_run_id,
                    call_key,
                    input_fingerprint_json,
                )
                .await?
            {
                let checkpoint_json =
                    workflow::required_llm_call_checkpoint_json(checkpoint, call_key)
                        .map_err(workflow_checkpoint_contract_app_error)?;
                let (optimized, usage, finish_reason) =
                    restored_manga_prompt_optimization_checkpoint(&checkpoint_json, call_key)?;
                (optimized, None, usage, finish_reason)
            } else {
                let repair_request = request.clone();
                let response = self
                    .stream_complete_llm(request)
                    .await
                    .map_err(|err| AppError::LlmError(format!("优化漫画提示词失败: {err}")))?;
                if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                    return Err(AppError::InvalidArgument(
                        "优化漫画提示词输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“媒体提示词”输出档位，或缩短本次提示词后重试".to_string(),
                    ));
                }
                let checkpoint_response_text = response.message.content.clone();
                let response_usage_json = response
                    .usage
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()?;
                let parse_output = |value: &Value| {
                    let optimized: OptimizedMangaPromptFields =
                        serde_json::from_value(value.clone()).map_err(|err| {
                            AppError::InvalidArgument(format!(
                                "漫画提示词优化结果字段不符合要求: {err}"
                            ))
                        })?;
                    validate_manga_prompt_optimization(optimized)
                };
                let repaired = self
                    .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
                        request: repair_request,
                        raw_response: &checkpoint_response_text,
                        label: "漫画提示词优化",
                        schema: json!({
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["image_prompt"],
                            "properties": {"image_prompt": {"type": "string"}}
                        }),
                        checkpoint: Some(StructuredRepairCheckpointContext {
                            workflow_run_id,
                            base_call_key: call_key.to_string(),
                            call_order: 1,
                        }),
                        cancel_token: None,
                        progress: None,
                        parse_output: &parse_output,
                    })
                    .await?;
                (
                    repaired.output,
                    Some(checkpoint_response_text),
                    response_usage_json,
                    response.finish_reason,
                )
            };

        let ValidatedMangaPromptOptimization {
            parsed_image_prompt,
            image_prompt,
        } = validated;
        if let Some(checkpoint_response_text) = checkpoint_response_text {
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: call_key.to_string(),
                call_order: 1,
                input_fingerprint_json: input_fingerprint_json.clone(),
                checkpoint_json: json!({
                    "output": {"image_prompt": image_prompt},
                    "parsed_output": {"image_prompt": parsed_image_prompt},
                    "response_content": checkpoint_response_text,
                    "usage": response_usage_json,
                    "finish_reason": response_finish_reason,
                }),
                artifacts_json: None,
            })
            .await?;
        }
        Ok(image_prompt)
    }

    async fn finalize_manga_prompt_optimization(
        &self,
        workflow_run_id: i64,
        result: MangaPromptOptimizationResult,
    ) -> MangaPromptOptimizationResult {
        let (success, error_message) = match &result {
            Ok(_) => (true, None),
            Err(error) => (false, Some(error.to_string())),
        };
        let status_result = self
            .finalize_workflow_run_validated(workflow_run_id, success, error_message.as_deref())
            .await;
        match (result, status_result) {
            (result, Ok(_)) => result,
            (Ok(_), Err(status_error)) => Err(AppError::Internal(anyhow::anyhow!(
                "漫画提示词优化完成，但工作流状态写入失败: {status_error}"
            ))),
            (Err(primary), Err(status_error)) => Err(AppError::Internal(anyhow::anyhow!(
                "{primary}；工作流失败状态写入失败: {status_error}"
            ))),
        }
    }
}

pub(super) fn required_manga_prompt_optimization_input(
    input: MangaPromptOptimizationInput,
) -> Result<RequiredMangaPromptOptimizationInput, AppError> {
    if input.panel.id <= 0 {
        return Err(AppError::InvalidArgument(
            "漫画提示词 ID 必须大于 0".to_string(),
        ));
    }
    if input.panel.project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    if input.panel.chapter_id <= 0 {
        return Err(AppError::InvalidArgument("章节 ID 必须大于 0".to_string()));
    }
    if input.panel.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err(AppError::InvalidArgument(
            "只能优化整页漫画提示词".to_string(),
        ));
    }
    let instruction = required_text(&input.instruction, "优化要求")
        .map_err(AppError::InvalidArgument)?
        .to_string();
    if let Some(image_id) = input
        .downstream_image_ids
        .iter()
        .find(|image_id| **image_id <= 0)
    {
        return Err(AppError::InvalidArgument(format!(
            "下游图片 ID 必须大于 0，实际为 {image_id}"
        )));
    }
    Ok(RequiredMangaPromptOptimizationInput {
        panel: input.panel,
        instruction,
        downstream_image_ids: input.downstream_image_ids,
    })
}
