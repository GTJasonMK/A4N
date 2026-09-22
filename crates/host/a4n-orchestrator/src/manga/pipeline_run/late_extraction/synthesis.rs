use a4n_error::AppError;
use serde_json::Value;

use super::super::super::MangaPipelineStage;
use super::super::super::pipeline_prompt::{
    MangaRenderTraceInput, manga_chapter_comic_synthesis_user_message,
    validate_manga_chapter_comic_synthesis,
};
use super::super::contracts::MangaRunContext;

pub(super) struct ComicSynthesisStageInput<'a> {
    pub(super) events_json: &'a Value,
    pub(super) speech_json: &'a Value,
    pub(super) visual_json: &'a Value,
    pub(super) items_json: &'a Value,
}

pub(super) async fn run_comic_synthesis_stage(
    context: &MangaRunContext<'_>,
    input: ComicSynthesisStageInput<'_>,
) -> Result<Value, AppError> {
    let ComicSynthesisStageInput {
        events_json,
        speech_json,
        visual_json,
        items_json,
    } = input;
    let synthesis_user = manga_chapter_comic_synthesis_user_message(
        events_json.clone(),
        speech_json.clone(),
        visual_json.clone(),
        items_json.clone(),
    )?;
    let (comic_synthesis_trace, synthesis_json) = context
        .service
        .render_and_validate_manga_call(
            MangaRenderTraceInput {
                prompt_renderer: context.prompt_renderer,
                prompt_name: "manga_chapter_comic_synthesis",
                project_id: context.project_id,
                workflow_run_id: context.workflow_run_id,
                call_key: format!(
                    "manga_pipeline:{}:manga_chapter_comic_synthesis",
                    context.chapter_id
                ),
                call_order: 5,
                restore_from_checkpoint: context.resume_from_checkpoint,
                variables: &context.variables,
                user_content: synthesis_user,
                temperature: 0.3,
            },
            |value| {
                validate_manga_chapter_comic_synthesis(value)?;
                Ok(value.clone())
            },
        )
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.workflow_run_id,
            "comic_synthesis",
            5,
            context.task_id,
            &comic_synthesis_trace,
        )
        .await?;
    context
        .advance_stage(MangaPipelineStage::PagePlanning)
        .await?;

    Ok(synthesis_json)
}
