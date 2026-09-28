use a4n_error::AppError;

use super::super::super::super::MANGA_PIPELINE_TOTAL_STEPS;
use super::super::contracts::{MangaPlanningRun, MangaRunContext};
use super::super::drawing_prompt::DrawingPromptStepInput;
use super::contracts::MangaCompletionArtifacts;

pub(super) async fn run_completion_drawing(
    context: &MangaRunContext<'_>,
    planning: MangaPlanningRun,
) -> Result<MangaCompletionArtifacts, AppError> {
    let service = context.service;
    let chapter_id = context.chapter_id;
    let project_id = context.project_id;
    let reference_assets = context.reference_assets.clone();
    let MangaPlanningRun {
        extraction,
        plan,
        page_plan_ref,
    } = planning;
    let extraction_summary = extraction.extraction_summary;

    let dp_step = service
        .run_page_drawing_prompt_step(DrawingPromptStepInput {
            chapter_id,
            project_id,
            prompt_renderer: context.prompt_renderer,
            vars: &context.variables,
            task_id: context.task_id,
            workflow_run_id: context.workflow_run_id,
            reference_assets: &reference_assets,
            plan: &plan,
            extraction_summary: &extraction_summary,
            restore_from_checkpoint: context.resume_from_checkpoint,
        })
        .await?;
    let page_prompts = dp_step.page_prompts;
    let page_traces = dp_step.page_traces;
    service
        .record_manga_pipeline_progress(
            context.workflow_run_id,
            context.task_id,
            "persisting",
            MANGA_PIPELINE_TOTAL_STEPS,
            "正在写入整页漫画提示词",
        )
        .await?;

    Ok(MangaCompletionArtifacts {
        plan,
        page_plan_ref,
        page_prompts,
        page_traces,
    })
}
