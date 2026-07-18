use a4n_core::error::AppError;
use serde_json::json;

use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY, MangaContextManifestInput,
    MangaContextPreviousOutput, MangaPromptGenerationTrace, MangaRenderTraceInput,
    build_manga_context_manifest, manga_page_planning_user_message,
};
use super::super::contracts::{MangaExtractionRun, MangaRunContext};

pub(super) async fn generate_planning_trace(
    context: &MangaRunContext<'_>,
    extraction: &MangaExtractionRun,
) -> Result<MangaPromptGenerationTrace, AppError> {
    let extraction_summary = &extraction.extraction_summary;
    let reference_assets = context.reference_assets.clone();
    let shooting_script = json!({
        "shooting_script_id": context.shooting_script.record.id,
        "shooting_script_revision": context.shooting_script.record.revision,
        "script": context.shooting_script.script,
    });
    let plan_user = manga_page_planning_user_message(extraction_summary.clone(), shooting_script)?;
    let planning_manifest = build_manga_context_manifest(MangaContextManifestInput {
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        chapter_content: &context.chapter_content,
        source_spans: &context.source_spans,
        prompt_names: &[
            "shooting_script_projection",
            "manga_character_binding",
            "manga_speech_selection",
            "manga_visual_anchor_extraction",
            "manga_item_extraction",
            "manga_chapter_comic_synthesis",
            "manga_page_planning",
        ],
        previous_outputs: &[MangaContextPreviousOutput {
            pack_kind: MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY,
            prompt_name: "manga_extraction_summary",
            parsed_output: extraction_summary,
            raw_output: None,
        }],
        reference_assets: Some(&reference_assets),
        dynamic_user_content: plan_user,
    })
    .await?;

    context
        .service
        .render_and_chat_trace(MangaRenderTraceInput {
            prompt_renderer: context.prompt_renderer,
            prompt_name: "manga_page_planning",
            project_id: context.project_id,
            workflow_run_id: context.workflow_run_id,
            call_key: format!("manga_pipeline:{}:manga_page_planning", context.chapter_id),
            call_order: 6,
            restore_from_checkpoint: context.should_restore_stage(6),
            variables: &context.variables,
            manifest: planning_manifest,
            temperature: 0.3,
        })
        .await
}
