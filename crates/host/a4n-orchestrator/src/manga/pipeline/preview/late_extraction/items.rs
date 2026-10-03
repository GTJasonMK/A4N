use a4n_error::AppError;
use serde_json::Value;

use super::super::super::prompt::{
    manga_item_available_source_span_ids, manga_item_extraction_user_message,
    validate_manga_item_extraction_source_span_ids,
};
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) struct ItemPreviewInput<'a> {
    pub(super) speech_json: &'a Value,
    pub(super) visual_json: &'a Value,
}

pub(super) async fn preview_items(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    input: ItemPreviewInput<'_>,
) -> Result<Option<Value>, AppError> {
    let items_call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_item_extraction",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_item_extraction",
            context.chapter_id
        ),
        call_order: 4,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_item_extraction_user_message(
            input.speech_json.clone(),
            input.visual_json.clone(),
            &context.source_markers,
        )?,
        temperature: 0.3,
    })
    .await?;
    let items_json = match items_call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_item_extraction",
                "漫画 Step 4：关键物品抽取",
                "manga_item_extraction",
            ));
            parsed_output
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_item_extraction",
                "漫画 Step 4：关键物品抽取",
                "manga_item_extraction",
                None,
                *request,
            ));
            return Ok(None);
        }
    };
    let allowed_source_span_ids =
        manga_item_available_source_span_ids(input.speech_json, input.visual_json)?;
    validate_manga_item_extraction_source_span_ids(
        &items_json,
        &context.source_spans,
        &allowed_source_span_ids,
        &crate::manga::adaptation_projection::manga_beat_refs(&context.adaptation.plan),
    )?;

    Ok(Some(items_json))
}
