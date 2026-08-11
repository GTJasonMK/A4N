use super::super::super::pipeline_json::page_drawing_prompt_from_response;
use super::super::super::pipeline_prompt::{
    MangaRenderTraceInput, manga_page_drawing_prompt_user_message,
    parse_and_validate_manga_llm_output,
};
use super::super::super::pipeline_state::manga_drawing_prompt_call_order;
use super::super::super::{MANGA_PIPELINE_TOTAL_STEPS, OrchestratorContext};
use super::contracts::{SinglePageDrawingPromptInput, SinglePageDrawingPromptOutput};
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub(super) async fn run_single_page_drawing_prompt(
        &self,
        input: SinglePageDrawingPromptInput<'_>,
    ) -> Result<SinglePageDrawingPromptOutput, AppError> {
        let SinglePageDrawingPromptInput {
            chapter_id,
            project_id,
            prompt_renderer,
            vars,
            task_id,
            workflow_run_id,
            reference_assets,
            page_plan,
            page_index,
            total_pages,
            extraction_summary,
            restore_from_checkpoint,
        } = input;
        let page_number = page_plan.page_number;
        let page_context = format!("manga_page_drawing_prompt.page_{page_number}");
        let dp_user = manga_page_drawing_prompt_user_message(
            page_number,
            page_plan,
            extraction_summary,
            reference_assets,
        )?;
        let call_order = manga_drawing_prompt_call_order(page_index)?;
        let trace = self
            .render_and_chat_trace(MangaRenderTraceInput {
                prompt_renderer,
                prompt_name: "manga_page_drawing_prompt",
                project_id,
                workflow_run_id,
                call_key: format!(
                    "manga_pipeline:{chapter_id}:manga_page_drawing_prompt:page:{page_number}"
                ),
                call_order,
                restore_from_checkpoint,
                variables: vars,
                user_content: dp_user,
                temperature: 0.5,
            })
            .await?;
        self.report_pipeline_llm_usage(
            workflow_run_id,
            "drawing_prompt",
            MANGA_PIPELINE_TOTAL_STEPS,
            task_id,
            &trace,
        )
        .await?;
        let page_prompt = parse_and_validate_manga_llm_output(&trace, |value| {
            page_drawing_prompt_from_response(value, page_number, &page_context)
        })?;
        let page_json = serde_json::to_value(&page_prompt).map_err(AppError::Serialization)?;
        self.save_or_validate_manga_llm_checkpoint(&trace, &page_json)
            .await?;
        let message = format!(
            "已完成第 {page_number} 页整页提示词 ({}/{total_pages})",
            page_index + 1
        );
        self.record_manga_pipeline_progress(
            workflow_run_id,
            task_id,
            "drawing_prompt",
            MANGA_PIPELINE_TOTAL_STEPS,
            &message,
        )
        .await?;

        Ok(SinglePageDrawingPromptOutput {
            prompt: page_prompt,
            trace,
        })
    }
}
