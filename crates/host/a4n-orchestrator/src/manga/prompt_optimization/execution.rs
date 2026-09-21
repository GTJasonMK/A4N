use a4n_api_contract::manga::MangaPromptDownstreamCleanupPort;
use a4n_db_models::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use a4n_domain::project::required_text;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_prompt::rendering::PromptRenderer;
use serde::Deserialize;

use crate::{
    OrchestratorContext, manga::ensure_manga_source_can_write,
    structured_llm::parse_strict_json_response,
};

use super::{
    contracts::{
        MangaPromptOptimizationInput, PreparedMangaPromptOptimization,
        RequiredMangaPromptOptimizationInput,
    },
    request::build_manga_prompt_optimization_request,
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
        cleanup_port: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        let prepared = self
            .prepare_manga_prompt_optimization(input, prompt_renderer)
            .await?;
        self.execute_manga_prompt_optimization(prepared, cleanup_port)
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
            downstream_image_ids: required.downstream_image_ids,
            request,
        })
    }

    async fn execute_manga_prompt_optimization(
        &self,
        prepared: PreparedMangaPromptOptimization,
        cleanup_port: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        let image_prompt = self
            .run_manga_prompt_optimization_call(prepared.request)
            .await?;
        if prepared.page_prompt.image_prompt.trim() == image_prompt {
            return Ok(prepared.page_prompt);
        }
        let updated = self
            .update_manga_page_prompt_image_prompt(prepared.page_prompt, image_prompt)
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
    ) -> Result<String, AppError> {
        let response = self
            .stream_complete_llm(request)
            .await
            .map_err(manga_prompt_optimization_llm_error)?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "优化漫画提示词输出达到当前 max_tokens 上限，返回 JSON 被截断；请在模型配置中提高 max_output_tokens，或缩短本次提示词后重试".to_string(),
            ));
        }
        let parsed = parse_strict_json_response::<OptimizedMangaPromptFields>(
            &response.message.content,
            "漫画提示词优化",
        )?;
        required_text(&parsed.image_prompt, "优化结果 image_prompt")
            .map(str::to_string)
            .map_err(AppError::InvalidArgument)
    }
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
        page_prompt: input.page_prompt,
        instruction,
        downstream_image_ids: input.downstream_image_ids,
    })
}
