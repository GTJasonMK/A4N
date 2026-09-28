use a4n_error::AppError;
use serde_json::Value;

use super::super::super::prompt::{
    manga_visual_anchor_extraction_user_message, validate_manga_visual_anchor_source_span_ids,
};
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_visual(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    events_json: &Value,
) -> Result<Option<Value>, AppError> {
    let visual_call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_visual_anchor_extraction",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_visual_anchor_extraction",
            context.chapter_id
        ),
        call_order: 3,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_visual_anchor_extraction_user_message(
            events_json.clone(),
            &context.source_spans,
        )?,
        temperature: 0.3,
    })
    .await?;
    let visual_json = match visual_call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_visual_anchor_extraction",
                "漫画 Step 3：视觉锚点抽取",
                "manga_visual_anchor_extraction",
            ));
            parsed_output
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_visual_anchor_extraction",
                "漫画 Step 3：视觉锚点抽取",
                "manga_visual_anchor_extraction",
                None,
                *request,
            ));
            return Ok(None);
        }
    };
    validate_manga_visual_anchor_source_span_ids(
        &visual_json,
        &context.source_spans,
        &crate::manga::adaptation_projection::manga_beat_refs(&context.adaptation.plan),
    )?;

    Ok(Some(visual_json))
}
