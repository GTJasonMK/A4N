use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING, MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
    MangaContextManifestInput, MangaContextPreviousOutput, manga_speech_selection_user_message,
    validate_manga_speech_selection_source_span_ids,
};
use super::super::call::{build_manga_preview_call, push_blocked_after};
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) struct SpeechPreviewInput<'a> {
    pub(super) events_out: &'a str,
    pub(super) events_json: &'a Value,
    pub(super) characters_out: &'a str,
    pub(super) characters_json: &'a Value,
    pub(super) reference_assets: &'a Value,
}

pub(super) async fn preview_speech(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    input: SpeechPreviewInput<'_>,
) -> Result<Option<(String, Value)>, AppError> {
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_speech_selection",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_speech_selection",
            context.chapter_id
        ),
        call_order: 2,
        restore_from_checkpoint: context.should_restore_stage(2),
        manifest: MangaContextManifestInput {
            prompt_renderer: context.prompt_renderer,
            project_id: context.project_id,
            variables: &context.variables,
            chapter_content: &context.chapter_content,
            source_spans: &context.source_spans,
            prompt_names: &["manga_character_binding", "manga_speech_selection"],
            previous_outputs: &[
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
                    prompt_name: "shooting_script_projection",
                    parsed_output: &json!({ "events": input.events_json }),
                    raw_output: Some(input.events_out),
                },
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING,
                    prompt_name: "manga_character_binding",
                    parsed_output: &json!({ "characters": input.characters_json }),
                    raw_output: Some(input.characters_out),
                },
            ],
            reference_assets: Some(input.reference_assets),
            dynamic_user_content: manga_speech_selection_user_message(
                input.characters_json.clone(),
                input.events_json.clone(),
                &context.source_spans,
                input.reference_assets.clone(),
            )?,
        },
        temperature: 0.3,
    })
    .await?;
    let (speech_out, speech_json) = match call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_speech_selection",
                "漫画 Step 2：对白旁白选择",
                "manga_speech_selection",
            ));
            (serde_json::to_string(&parsed_output)?, parsed_output)
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_speech_selection",
                "漫画 Step 2：对白旁白选择",
                "manga_speech_selection",
                None,
                *request,
            ));
            push_blocked_after(steps, "manga_visual_anchor_extraction");
            return Ok(None);
        }
    };
    validate_manga_speech_selection_source_span_ids(&speech_json, &context.source_spans)?;

    Ok(Some((speech_out, speech_json)))
}
