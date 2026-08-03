use a4n_core::error::AppError;
use serde_json::json;

use super::super::super::OrchestratorContext;
use super::super::super::checkpoint::checkpoint_insert;
use super::super::super::pipeline_json::page_plan_pages;
use super::super::super::pipeline_persist::PageMangaPromptGenerationTrace;
use super::contracts::SinglePageDrawingPromptInput;
use super::{DrawingPromptStepInput, DrawingPromptStepOutput};

impl OrchestratorContext {
    pub(in crate::manga::pipeline_run) async fn run_page_drawing_prompt_step(
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
            source_metadata,
            reference_assets_metadata,
            prompt_fingerprint,
            checkpoint,
            chapter_content,
            source_spans,
            reference_assets,
            plan_json,
            extraction_summary,
            restore_from_checkpoint,
        } = input;
        let pages = page_plan_pages(plan_json, "manga_page_planning")?;
        let mut drawing_pages = Vec::with_capacity(pages.len());
        let mut page_traces = Vec::new();
        for (index, page_plan) in pages.iter().enumerate() {
            let page_output = self
                .run_single_page_drawing_prompt(SinglePageDrawingPromptInput {
                    chapter_id,
                    project_id,
                    prompt_renderer,
                    vars,
                    task_id,
                    workflow_run_id,
                    source_metadata,
                    reference_assets_metadata,
                    prompt_fingerprint,
                    checkpoint,
                    chapter_content,
                    source_spans,
                    reference_assets,
                    page_plan,
                    page_index: index,
                    total_pages: pages.len(),
                    extraction_summary,
                    restore_from_checkpoint,
                })
                .await?;
            page_traces.push(PageMangaPromptGenerationTrace {
                page_number: page_output.page_number,
                trace: page_output.trace,
            });
            drawing_pages.push(page_output.prompt);
        }
        checkpoint_insert(checkpoint, "dp_page_traces", json!(page_traces));
        let json = json!({ "pages": drawing_pages });

        Ok(DrawingPromptStepOutput { json, page_traces })
    }
}
