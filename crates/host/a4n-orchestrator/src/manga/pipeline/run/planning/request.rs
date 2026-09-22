use a4n_error::AppError;

use super::super::super::super::shooting_script_projection::manga_page_planning_source;
use super::super::super::prompt::{
    MangaPromptGenerationTrace, MangaRenderTraceInput, manga_page_planning_user_message,
};
use super::super::contracts::{MangaExtractionRun, MangaRunContext};

pub(super) async fn generate_planning_trace(
    context: &MangaRunContext<'_>,
    extraction: &MangaExtractionRun,
) -> Result<MangaPromptGenerationTrace, AppError> {
    let extraction_summary = &extraction.extraction_summary;
    let shooting_script = manga_page_planning_source(
        context.shooting_script.record.id,
        context.shooting_script.record.revision,
        &context.shooting_script.script,
    );
    let plan_user = manga_page_planning_user_message(extraction_summary.clone(), shooting_script)?;

    context
        .service
        .render_and_chat_trace(MangaRenderTraceInput {
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
        })
        .await
}
