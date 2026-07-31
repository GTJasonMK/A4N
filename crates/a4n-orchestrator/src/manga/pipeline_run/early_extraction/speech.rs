use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::MangaPipelineStartStage;
use super::super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING, MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
    MangaContextManifestInput, MangaContextPreviousOutput, MangaPromptGenerationTrace,
    MangaRenderTraceInput, build_manga_context_manifest, manga_speech_selection_user_message,
    validate_manga_speech_selection_source_span_ids,
};
use super::super::contracts::MangaRunContext;

pub(super) struct SpeechSelectionStageInput<'a> {
    pub(super) events_json: &'a Value,
    pub(super) events_out: &'a str,
    pub(super) characters_json: &'a Value,
    pub(super) characters_out: &'a str,
}

pub(super) async fn run_speech_selection_stage(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
    input: SpeechSelectionStageInput<'_>,
) -> Result<(MangaPromptGenerationTrace, String, Value), AppError> {
    let SpeechSelectionStageInput {
        events_json,
        events_out,
        characters_json,
        characters_out,
    } = input;
    let speech_user = manga_speech_selection_user_message(
        characters_json.clone(),
        events_json.clone(),
        &context.source_spans,
        context.reference_assets.clone(),
    )?;
    let events_context = json!({ "events": events_json });
    let characters_context = json!({ "characters": characters_json });
    let speech_manifest = build_manga_context_manifest(MangaContextManifestInput {
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
                parsed_output: &events_context,
                raw_output: Some(events_out),
            },
            MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING,
                prompt_name: "manga_character_binding",
                parsed_output: &characters_context,
                raw_output: Some(characters_out),
            },
        ],
        reference_assets: Some(&context.reference_assets),
        dynamic_user_content: speech_user,
    })
    .await?;
    let speech_trace = context
        .service
        .render_and_chat_trace(MangaRenderTraceInput {
            prompt_renderer: context.prompt_renderer,
            prompt_name: "manga_speech_selection",
            project_id: context.project_id,
            workflow_run_id: context.workflow_run_id,
            call_key: format!(
                "manga_pipeline:{}:manga_speech_selection",
                context.chapter_id
            ),
            call_order: 2,
            restore_from_checkpoint: context.should_restore_stage(2),
            variables: &context.variables,
            manifest: speech_manifest,
            temperature: 0.3,
        })
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.chapter_id,
            context.project_id,
            "speech_selection",
            2,
            context.task_id,
            &speech_trace,
        )
        .await?;
    let speech_out = speech_trace.response_text.clone();
    checkpoint_insert_trace(checkpoint, "speech_trace", &speech_trace)?;
    let speech_json = context
        .service
        .repair_and_validate_manga_llm_output(&speech_trace, |value| {
            validate_manga_speech_selection_source_span_ids(value, &context.source_spans)?;
            Ok(value.clone())
        })
        .await?;
    context
        .service
        .save_or_validate_manga_llm_checkpoint(&speech_trace, &speech_json)
        .await?;
    checkpoint_insert(checkpoint, "speech_out", json!(speech_out.clone()));
    context
        .checkpoint_and_advance_stage(
            checkpoint,
            MangaPipelineStartStage::SpeechSelection,
            "已完成对白旁白选择",
            MangaPipelineStartStage::VisualAnchorExtraction,
        )
        .await?;

    Ok((speech_trace, speech_out, speech_json))
}
