use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::PIPELINE_TOTAL_STEPS;
use super::super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION, MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
    MangaContextManifestInput, MangaContextPreviousOutput, MangaPromptGenerationTrace,
    MangaRenderTraceInput, build_manga_context_manifest, manga_item_available_source_span_ids,
    manga_item_extraction_user_message, validate_manga_item_extraction_source_span_ids,
};
use super::super::super::pipeline_status::MangaPipelineStatusInput;
use super::super::contracts::{MangaRunContext, MangaRunningPipelineCheckpointInput};

pub(super) async fn run_item_extraction_stage(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
    speech_json: &Value,
    speech_out: &str,
    visual_json: &Value,
    visual_out: &str,
) -> Result<(MangaPromptGenerationTrace, String, Value), AppError> {
    let items_user = manga_item_extraction_user_message(speech_json.clone(), visual_json.clone())?;
    let speech_context = json!({ "speech": speech_json });
    let visual_context = json!({ "visual_anchors": visual_json });
    let items_manifest = build_manga_context_manifest(MangaContextManifestInput {
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
                parsed_output: &speech_context,
                raw_output: Some(speech_out),
            },
            MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
                prompt_name: "manga_visual_anchor_extraction",
                parsed_output: &visual_context,
                raw_output: Some(visual_out),
            },
        ],
        reference_assets: Some(&context.reference_assets),
        dynamic_user_content: items_user,
    })
    .await?;
    let items_trace = context
        .service
        .render_and_chat_trace(MangaRenderTraceInput {
            prompt_renderer: context.prompt_renderer,
            prompt_name: "manga_item_extraction",
            project_id: context.project_id,
            workflow_run_id: context.workflow_run_id,
            call_key: format!(
                "manga_pipeline:{}:manga_item_extraction",
                context.chapter_id
            ),
            call_order: 4,
            restore_from_checkpoint: context.should_restore_stage(4),
            variables: &context.variables,
            manifest: items_manifest,
            temperature: 0.3,
        })
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.chapter_id,
            context.project_id,
            "item_extraction",
            4,
            context.task_id,
            &items_trace,
        )
        .await?;
    let items_out = items_trace.response_text.clone();
    checkpoint_insert_trace(checkpoint, "items_trace", &items_trace)?;
    let allowed_source_span_ids = manga_item_available_source_span_ids(speech_json, visual_json);
    let items_json = context
        .service
        .repair_and_validate_manga_llm_output(&items_trace, |value| {
            validate_manga_item_extraction_source_span_ids(
                value,
                &context.source_spans,
                &allowed_source_span_ids,
            )?;
            Ok(value.clone())
        })
        .await?;
    context
        .service
        .save_or_validate_manga_llm_checkpoint(&items_trace, &items_json)
        .await?;
    checkpoint_insert(checkpoint, "items_out", json!(items_out.clone()));
    context
        .service
        .save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id: context.chapter_id,
            project_id: context.project_id,
            stage: "item_extraction",
            current: 4,
            message: "已完成关键物品抽取",
            task_id: context.task_id,
            source_metadata: &context.source_metadata,
            reference_assets_metadata: &context.reference_assets_metadata,
            prompt_fingerprint: &context.prompt_fingerprint,
            checkpoint,
        })
        .await?;
    context
        .service
        .ensure_pipeline_not_cancelled(context.project_id, context.chapter_id)
        .await?;
    context
        .service
        .set_pipeline_status(MangaPipelineStatusInput {
            chapter_id: context.chapter_id,
            project_id: context.project_id,
            pipeline_status: "running",
            stage: "comic_synthesis",
            current: 5,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在综合漫画章节节奏",
            task_id: context.task_id,
            allow_cancel_override: false,
        })
        .await?;

    Ok((items_trace, items_out, items_json))
}
