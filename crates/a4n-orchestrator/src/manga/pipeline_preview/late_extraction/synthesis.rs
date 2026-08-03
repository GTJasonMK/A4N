use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION, MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
    MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION, MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
    MangaContextManifestInput, MangaContextPreviousOutput,
    manga_chapter_comic_synthesis_user_message, validate_manga_chapter_comic_synthesis,
};
use super::super::call::{build_manga_preview_call, push_blocked_after};
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) struct SynthesisPreviewInput<'a> {
    pub(super) events_out: &'a str,
    pub(super) events_json: &'a Value,
    pub(super) speech_out: &'a str,
    pub(super) speech_json: &'a Value,
    pub(super) visual_out: &'a str,
    pub(super) visual_json: &'a Value,
    pub(super) items_out: &'a str,
    pub(super) items_json: &'a Value,
    pub(super) reference_assets: &'a Value,
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
        restore_from_checkpoint: context.should_restore_stage(5),
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
                "manga_chapter_comic_synthesis",
            ],
            previous_outputs: &[
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
                    prompt_name: "shooting_script_projection",
                    parsed_output: &json!({ "events": input.events_json }),
                    raw_output: Some(input.events_out),
                },
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
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION,
                    prompt_name: "manga_item_extraction",
                    parsed_output: &json!({ "items": input.items_json }),
                    raw_output: Some(input.items_out),
                },
            ],
            reference_assets: Some(input.reference_assets),
            dynamic_user_content: manga_chapter_comic_synthesis_user_message(
                input.events_json.clone(),
                input.speech_json.clone(),
                input.visual_json.clone(),
                input.items_json.clone(),
            )?,
        },
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
            push_blocked_after(steps, "manga_page_planning");
            return Ok(None);
        }
    };
    validate_manga_chapter_comic_synthesis(&comic_synthesis_json)?;

    Ok(Some(comic_synthesis_json))
}
