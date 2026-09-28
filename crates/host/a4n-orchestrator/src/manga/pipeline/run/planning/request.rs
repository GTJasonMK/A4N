use super::super::super::json::parse_manga_page_plan;
use a4n_domain::manga::MangaPagePlanV4;
use a4n_error::AppError;

use super::super::super::super::adaptation_projection::manga_page_planning_source;
use super::super::super::prompt::{
    MangaPromptGenerationTrace, MangaRenderTraceInput, manga_page_planning_user_message,
};
use super::super::contracts::{MangaExtractionRun, MangaRunContext};

pub(super) async fn generate_planning_trace(
    context: &MangaRunContext<'_>,
    extraction: &MangaExtractionRun,
) -> Result<(MangaPromptGenerationTrace, MangaPagePlanV4), AppError> {
    let extraction_summary = &extraction.extraction_summary;
    let plan_user = manga_page_planning_user_message(
        extraction_summary.clone(),
        manga_page_planning_source(&context.adaptation),
    )?;

    context
        .service
        .render_and_validate_manga_call(
            MangaRenderTraceInput {
                prompt_renderer: context.prompt_renderer,
                prompt_name: "manga_page_planning",
                project_id: context.project_id,
                workflow_run_id: context.workflow_run_id,
                call_key: format!("manga_pipeline:{}:manga_page_planning", context.chapter_id),
                call_order: 6,
                restore_from_checkpoint: context.resume_from_checkpoint,
                variables: &context.variables,
                user_content: plan_user,
                temperature: 0.3,
            },
            |value| parse_manga_page_plan(value, &context.adaptation, "manga_page_planning"),
        )
        .await
}
