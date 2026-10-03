use a4n_error::AppError;
use serde_json::Value;

use super::super::super::prompt::{
    manga_chapter_comic_synthesis_user_message, validate_manga_chapter_comic_synthesis,
};
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) struct SynthesisPreviewInput<'a> {
    pub(super) events_json: &'a Value,
    pub(super) speech_json: &'a Value,
    pub(super) visual_json: &'a Value,
    pub(super) items_json: &'a Value,
}

pub(super) async fn preview_synthesis(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    input: SynthesisPreviewInput<'_>,
) -> Result<Option<Value>, AppError> {
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_chapter_comic_synthesis",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_chapter_comic_synthesis",
            context.chapter_id
        ),
        call_order: 5,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_chapter_comic_synthesis_user_message(
            input.events_json.clone(),
            input.speech_json.clone(),
            input.visual_json.clone(),
            input.items_json.clone(),
            &context.source_markers,
        )?,
        temperature: 0.3,
    })
    .await?;
    let comic_synthesis_json = match call {
        MangaPreviewCall::Restored { parsed_output, .. } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_chapter_comic_synthesis",
                "漫画 Step 5：章节节奏综合",
                "manga_chapter_comic_synthesis",
            ));
            parsed_output
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_chapter_comic_synthesis",
                "漫画 Step 5：章节节奏综合",
                "manga_chapter_comic_synthesis",
                None,
                *request,
            ));
            return Ok(None);
        }
    };
    validate_manga_chapter_comic_synthesis(&comic_synthesis_json)?;

    Ok(Some(comic_synthesis_json))
}
