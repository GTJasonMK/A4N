use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::MangaPipelineStartStage;
use super::super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION, MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
    MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION, MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
    MangaContextManifestInput, MangaContextPreviousOutput, MangaPromptGenerationTrace,
    MangaRenderTraceInput, build_manga_context_manifest,
    manga_chapter_comic_synthesis_user_message, validate_manga_chapter_comic_synthesis,
};
use super::super::contracts::MangaRunContext;

pub(super) struct ComicSynthesisStageInput<'a> {
    pub(super) events_json: &'a Value,
    pub(super) events_out: &'a str,
    pub(super) speech_json: &'a Value,
    pub(super) speech_out: &'a str,
    pub(super) visual_json: &'a Value,
    pub(super) visual_out: &'a str,
    pub(super) items_json: &'a Value,
    pub(super) items_out: &'a str,
}

pub(super) async fn run_comic_synthesis_stage(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
    input: ComicSynthesisStageInput<'_>,
) -> Result<(MangaPromptGenerationTrace, Value), AppError> {
    let ComicSynthesisStageInput {
        events_json,
        events_out,
        speech_json,
        speech_out,
        visual_json,
        visual_out,
        items_json,
        items_out,
    } = input;
    let synthesis_user = manga_chapter_comic_synthesis_user_message(
        events_json.clone(),
        speech_json.clone(),
        visual_json.clone(),
        items_json.clone(),
    )?;
    let events_context = json!({ "events": events_json });
    let speech_context = json!({ "speech": speech_json });
    let visual_context = json!({ "visual_anchors": visual_json });
    let items_context = json!({ "items": items_json });
    let synthesis_manifest = build_manga_context_manifest(MangaContextManifestInput {
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
                parsed_output: &events_context,
                raw_output: Some(events_out),
            },
            MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION,
                prompt_name: "manga_speech_selection",
                parsed_output: &speech_context,
                raw_output: Some(speech_out),
            },
            MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
                prompt_name: "manga_visual_anchor_extraction",
                parsed_output: &visual_context,
                raw_output: Some(visual_out),
            },
            MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION,
                prompt_name: "manga_item_extraction",
                parsed_output: &items_context,
                raw_output: Some(items_out),
            },
        ],
        reference_assets: Some(&context.reference_assets),
        dynamic_user_content: synthesis_user,
    })
    .await?;
    let comic_synthesis_trace = context
        .service
        .render_and_chat_trace(MangaRenderTraceInput {
            prompt_renderer: context.prompt_renderer,
            prompt_name: "manga_chapter_comic_synthesis",
            project_id: context.project_id,
            workflow_run_id: context.workflow_run_id,
            call_key: format!(
                "manga_pipeline:{}:manga_chapter_comic_synthesis",
                context.chapter_id
            ),
            call_order: 5,
            restore_from_checkpoint: context.should_restore_stage(5),
            variables: &context.variables,
            manifest: synthesis_manifest,
            temperature: 0.3,
        })
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.chapter_id,
            context.project_id,
            "comic_synthesis",
            5,
            context.task_id,
            &comic_synthesis_trace,
        )
        .await?;
    let comic_synthesis_out = comic_synthesis_trace.response_text.clone();
    checkpoint_insert_trace(checkpoint, "comic_synthesis_trace", &comic_synthesis_trace)?;
    let synthesis_json = context
        .service
        .repair_and_validate_manga_llm_output(&comic_synthesis_trace, |value| {
            validate_manga_chapter_comic_synthesis(value)?;
            Ok(value.clone())
        })
        .await?;
    context
        .service
        .save_or_validate_manga_llm_checkpoint(&comic_synthesis_trace, &synthesis_json)
        .await?;
    checkpoint_insert(
        checkpoint,
        "comic_synthesis_out",
        json!(comic_synthesis_out),
    );
    context
        .checkpoint_and_advance_stage(
            checkpoint,
            MangaPipelineStartStage::ComicSynthesis,
            "已完成漫画章节综合",
            MangaPipelineStartStage::PagePlanning,
        )
        .await?;

    Ok((comic_synthesis_trace, synthesis_json))
}
