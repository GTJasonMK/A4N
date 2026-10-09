use super::super::super::super::OrchestratorContext;
use super::super::super::json::page_drawing_prompt_from_response;
use super::super::super::prompt::{
    MangaRenderTraceInput, manga_page_drawing_prompt_pipeline_user_message,
};
use super::super::super::state::manga_page_pipeline_call_order;
use super::contracts::{SinglePageDrawingPromptInput, SinglePageDrawingPromptOutput};
use a4n_error::AppError;

impl OrchestratorContext {
    pub(in crate::manga::pipeline::run) async fn run_single_page_drawing_prompt(
        &self,
        input: SinglePageDrawingPromptInput<'_>,
    ) -> Result<SinglePageDrawingPromptOutput, AppError> {
        let SinglePageDrawingPromptInput {
            chapter_id,
            project_id,
            prompt_renderer,
            vars,
            workflow_run_id,
            reference_assets,
            reference_bindings,
            system_reference_images,
            available_reference_images,
            page_content,
            page_adaptation,
            page_plan,
            page_index,
            extraction_summary,
            source_markers,
            restore_from_checkpoint,
            cancel_token,
            stream_reporter,
        } = input;
        let page_number = page_plan.page_number;
        let page_context = format!("manga_page_drawing_prompt.page_{page_number}");
        let dp_user = manga_page_drawing_prompt_pipeline_user_message(
            page_number,
            page_plan,
            page_content,
            page_adaptation,
            extraction_summary,
            reference_assets,
            reference_bindings,
            system_reference_images,
            available_reference_images,
            source_markers,
        )?;
        let call_order = manga_page_pipeline_call_order(page_index, 3)?;
        let (trace, page_prompt) = self
            .render_and_validate_manga_call(
                MangaRenderTraceInput {
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
                    cancel_token,
                    stream_id: format!("page_{page_number}_drawing_prompt"),
                    stream_reporter,
                },
                |value| page_drawing_prompt_from_response(value, page_number, &page_context),
            )
            .await?;

        Ok(SinglePageDrawingPromptOutput {
            prompt: page_prompt,
            trace,
        })
    }
}
