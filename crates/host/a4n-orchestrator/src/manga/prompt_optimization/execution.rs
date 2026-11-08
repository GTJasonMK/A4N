use a4n_db::manga::MANGA_PAGE_PROMPT_TYPE;
use a4n_domain::{manga::MangaPagePromptRecord, project::required_text};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_prompt::rendering::PromptRenderer;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    manga::ensure_manga_source_can_write,
    structured_llm::{ensure_checkpoint_parsed_output_matches, parse_strict_json_response},
    workflow::{
        CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput, UpsertWorkflowRunInput,
        WORKFLOW_STATUS_RUNNING,
    },
};

use super::{
    contracts::{
        MangaPromptOptimizationInput, PreparedMangaPromptOptimization,
        RequiredMangaPromptOptimizationInput,
    },
    request::{build_manga_prompt_optimization_request, manga_prompt_optimization_call_key},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OptimizedMangaPromptFields {
    pub(super) image_prompt: String,
}

impl OrchestratorContext {
    pub async fn optimize_manga_prompt_validated(
        &self,
        input: MangaPromptOptimizationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<MangaPagePromptRecord, AppError> {
        let prepared = self
            .prepare_manga_prompt_optimization(input, prompt_renderer)
            .await?;
        let fingerprint = a4n_llm_trace::context_aware_chat_request_trace_json(&prepared.request)?;
        let call_key = manga_prompt_optimization_call_key(prepared.page_prompt.id);
        let workflow = self
            .begin_or_resume_workflow_run_validated(UpsertWorkflowRunInput {
                business_key: format!("manga_prompt_optimization:{}", prepared.page_prompt.id),
                workflow_kind: "manga_prompt_optimization".into(),
                project_id: Some(prepared.page_prompt.project_id),
                root_project_id: None,
                source_snapshot_id: None,
                scope_table: Some("chapter_manga_prompt".into()),
                scope_id: Some(prepared.page_prompt.id),
                chapter_id: Some(prepared.page_prompt.chapter_id),
                mode: None,
                input_fingerprint_json: Some(fingerprint.clone()),
                status: WORKFLOW_STATUS_RUNNING.into(),
                current_step_key: Some(call_key),
                active_task_id: None,
                can_resume: true,
                resume_block_reason: None,
                error_message: None,
            })
            .await?;
        let result = self
            .execute_manga_prompt_optimization(workflow.id, &fingerprint, prepared)
            .await;
        self.finalize_workflow_result(workflow.id, result, "漫画提示词优化")
            .await
    }

    async fn prepare_manga_prompt_optimization(
        &self,
        input: MangaPromptOptimizationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<PreparedMangaPromptOptimization, AppError> {
        let required = required_manga_prompt_optimization_input(input)?;
        let source = self
            .chapter_manga_source_by_id(
                required.page_prompt.chapter_id,
                required.page_prompt.project_id,
                false,
            )
            .await?;
        ensure_manga_source_can_write(&source, "优化整页漫画提示词")?;
        let request = build_manga_prompt_optimization_request(
            prompt_renderer,
            &required.page_prompt,
            &required.instruction,
        )
        .await?;
        Ok(PreparedMangaPromptOptimization {
            page_prompt: required.page_prompt,
            request,
        })
    }

    async fn execute_manga_prompt_optimization(
        &self,
        workflow_run_id: i64,
        fingerprint: &Value,
        prepared: PreparedMangaPromptOptimization,
    ) -> Result<MangaPagePromptRecord, AppError> {
        let image_prompt = self
            .run_manga_prompt_optimization_call(
                workflow_run_id,
                &manga_prompt_optimization_call_key(prepared.page_prompt.id),
                fingerprint,
                prepared.request,
            )
            .await?;
        let mut page_prompt = prepared.page_prompt;
        page_prompt.image_prompt = image_prompt;
        Ok(page_prompt)
    }

    pub(super) async fn run_manga_prompt_optimization_call(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        fingerprint: &Value,
        mut request: ChatRequest,
    ) -> Result<String, AppError> {
        crate::workflow::attach_llm_call_checkpoint_trace(
            &mut request,
            workflow_run_id,
            call_key,
            fingerprint,
        )?;
        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, call_key, fingerprint)
            .await?
        {
            let parsed =
                parse_optimized_manga_prompt(&checkpoint.response_content).and_then(|parsed| {
                    ensure_checkpoint_parsed_output_matches(
                        &checkpoint.parsed_output,
                        &Value::String(parsed.clone()),
                        call_key,
                    )?;
                    Ok(parsed)
                });
            return self
                .validate_llm_call_output(workflow_run_id, call_key, fingerprint, parsed)
                .await;
        }
        let response = self
            .stream_complete_llm(request.clone())
            .await
            .map_err(manga_prompt_optimization_llm_error)?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "优化漫画提示词输出达到当前 max_tokens 上限，返回 JSON 被截断；请在模型配置中提高 max_output_tokens，或缩短本次提示词后重试".to_string(),
            ));
        }
        let (response, prompt) = self
            .validate_llm_response(
                &request,
                response,
                call_key,
                None,
                parse_optimized_manga_prompt,
            )
            .await?;
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: call_key.to_string(),
            call_order: 1,
            input_fingerprint_json: fingerprint.clone(),
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: Value::String(prompt.clone()),
                response_content: response.message.content,
                usage: response.usage.map(serde_json::to_value).transpose()?,
                finish_reason: response.finish_reason,
                artifacts_json: None,
            },
        })
        .await?;
        Ok(prompt)
    }
}

fn parse_optimized_manga_prompt(raw: &str) -> Result<String, AppError> {
    let parsed = parse_strict_json_response::<OptimizedMangaPromptFields>(raw, "漫画提示词优化")?;
    required_text(&parsed.image_prompt, "优化结果 image_prompt")
        .map(str::to_string)
        .map_err(AppError::InvalidArgument)
}

pub(super) fn manga_prompt_optimization_llm_error(error: AppError) -> AppError {
    match error {
        AppError::Cancelled => AppError::Cancelled,
        other => AppError::LlmError(format!("优化漫画提示词失败: {other}")),
    }
}

pub(super) fn required_manga_prompt_optimization_input(
    input: MangaPromptOptimizationInput,
) -> Result<RequiredMangaPromptOptimizationInput, AppError> {
    if input.page_prompt.id <= 0 {
        return Err(AppError::InvalidArgument(
            "漫画提示词 ID 必须大于 0".to_string(),
        ));
    }
    if input.page_prompt.project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    if input.page_prompt.chapter_id <= 0 {
        return Err(AppError::InvalidArgument("章节 ID 必须大于 0".to_string()));
    }
    if input.page_prompt.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err(AppError::InvalidArgument(
            "只能优化整页漫画提示词".to_string(),
        ));
    }
    let instruction = required_text(&input.instruction, "优化要求")
        .map_err(AppError::InvalidArgument)?
        .to_string();
    if input.expected_updated_at.trim() != input.page_prompt.updated_at.to_string() {
        return Err(AppError::InvalidArgument(
            "漫画提示词已被其他操作修改，请刷新后重试".to_string(),
        ));
    }
    Ok(RequiredMangaPromptOptimizationInput {
        page_prompt: input.page_prompt,
        instruction,
    })
}
