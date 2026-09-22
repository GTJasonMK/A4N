use super::super::super::super::OrchestratorContext;
use super::super::super::persist::PageMangaPromptGenerationTrace;
use super::contracts::SinglePageDrawingPromptInput;
use super::{DrawingPromptStepInput, DrawingPromptStepOutput};
use a4n_error::AppError;

impl OrchestratorContext {
    pub(in crate::manga::pipeline::run) async fn run_page_drawing_prompt_step(
        &self,
        input: DrawingPromptStepInput<'_>,
    ) -> Result<DrawingPromptStepOutput, AppError> {
        let DrawingPromptStepInput {
            chapter_id,
            project_id,
            prompt_renderer,
            vars,
            task_id,
            workflow_run_id,
            reference_assets,
            plan,
            extraction_summary,
            restore_from_checkpoint,
        } = input;
        let mut page_prompts = Vec::with_capacity(plan.pages.len());
        let mut page_traces = Vec::new();
        for (index, page_plan) in plan.pages.iter().enumerate() {
            let page_output = self
                .run_single_page_drawing_prompt(SinglePageDrawingPromptInput {
                    chapter_id,
                    project_id,
                    prompt_renderer,
                    vars,
                    task_id,
                    workflow_run_id,
                    reference_assets,
                    page_plan,
                    page_index: index,
                    total_pages: plan.pages.len(),
                    extraction_summary,
                    restore_from_checkpoint,
                })
                .await?;
            let page_number = page_output.prompt.page_number;
            page_traces.push(PageMangaPromptGenerationTrace {
                page_number,
                trace: page_output.trace,
            });
            page_prompts.push(page_output.prompt);
        }
        Ok(DrawingPromptStepOutput {
            page_prompts,
            page_traces,
        })
    }
}
