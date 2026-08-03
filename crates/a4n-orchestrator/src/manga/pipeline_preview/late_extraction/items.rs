use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION, MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
    MangaContextManifestInput, MangaContextPreviousOutput, manga_item_available_source_span_ids,
    manga_item_extraction_user_message, validate_manga_item_extraction_source_span_ids,
};
use super::super::call::{build_manga_preview_call, push_blocked_after};
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) struct ItemPreviewInput<'a> {
    pub(super) speech_out: &'a str,
    pub(super) speech_json: &'a Value,
    pub(super) visual_out: &'a str,
    pub(super) visual_json: &'a Value,
    pub(super) reference_assets: &'a Value,
}

pub(super) async fn preview_items(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    input: ItemPreviewInput<'_>,
) -> Result<Option<(String, Value)>, AppError> {
    let items_call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_item_extraction",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_item_extraction",
            context.chapter_id
        ),
        call_order: 4,
        restore_from_checkpoint: context.should_restore_stage(4),
        manifest: MangaContextManifestInput {
            prompt_renderer: context.prompt_renderer,
            project_id: context.project_id,
            variables: &context.variables,
            chapter_content: &context.chapter_content,
            source_spans: &context.source_spans,
            prompt_names: &[
                "manga_speech_selection",
                "manga_visual_anchor_extraction",
                "manga_item_extraction",
            ],
            previous_outputs: &[
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION,
                    prompt_name: "manga_speech_selection",
                    parsed_output: &json!({ "speech": input.speech_json }),
                    raw_output: Some(input.speech_out),
                },
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
                    prompt_name: "manga_visual_anchor_extraction",
                    parsed_output: &json!({ "visual_anchors": input.visual_json }),
                    raw_output: Some(input.visual_out),
                },
            ],
            reference_assets: Some(input.reference_assets),
            dynamic_user_content: manga_item_extraction_user_message(
                input.speech_json.clone(),
                input.visual_json.clone(),
            )?,
        },
        temperature: 0.3,
    })
    .await?;
    let (items_out, items_json) = match items_call {
        MangaPreviewCall::Restored {
            response_text,
            parsed_output,
        } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_item_extraction",
                "漫画 Step 4：关键物品抽取",
                "manga_item_extraction",
            ));
            (response_text, parsed_output)
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_item_extraction",
                "漫画 Step 4：关键物品抽取",
                "manga_item_extraction",
                None,
                *request,
            ));
            push_blocked_after(steps, "manga_chapter_comic_synthesis");
            return Ok(None);
        }
    };
    let allowed_source_span_ids =
        manga_item_available_source_span_ids(input.speech_json, input.visual_json);
    validate_manga_item_extraction_source_span_ids(
        &items_json,
        &context.source_spans,
        &allowed_source_span_ids,
    )?;

    Ok(Some((items_out, items_json)))
}
