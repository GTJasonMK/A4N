use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::PIPELINE_TOTAL_STEPS;
use super::super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS, MangaContextManifestInput,
    MangaContextPreviousOutput, MangaPromptGenerationTrace, MangaRenderTraceInput,
    build_manga_context_manifest, manga_visual_anchor_extraction_user_message,
    validate_manga_visual_anchor_source_span_ids,
};
use super::super::super::pipeline_status::MangaPipelineStatusInput;
use super::super::contracts::{MangaRunContext, MangaRunningPipelineCheckpointInput};

pub(super) async fn run_visual_anchor_stage(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
    events_json: &Value,
    events_out: &str,
) -> Result<(MangaPromptGenerationTrace, String, Value), AppError> {
    let visual_user =
        manga_visual_anchor_extraction_user_message(events_json.clone(), &context.source_spans)?;
    let events_context = json!({ "events": events_json });
    let visual_manifest = build_manga_context_manifest(MangaContextManifestInput {
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        chapter_content: &context.chapter_content,
        source_spans: &context.source_spans,
        prompt_names: &[
            "shooting_script_projection",
            "manga_visual_anchor_extraction",
        ],
        previous_outputs: &[MangaContextPreviousOutput {
            pack_kind: MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
            prompt_name: "shooting_script_projection",
            parsed_output: &events_context,
            raw_output: Some(events_out),
        }],
        reference_assets: Some(&context.reference_assets),
        dynamic_user_content: visual_user,
    })
    .await?;
    let visual_trace = context
        .service
        .render_and_chat_trace(MangaRenderTraceInput {
            prompt_renderer: context.prompt_renderer,
            prompt_name: "manga_visual_anchor_extraction",
            project_id: context.project_id,
            workflow_run_id: context.workflow_run_id,
            call_key: format!(
                "manga_pipeline:{}:manga_visual_anchor_extraction",
                context.chapter_id
            ),
            call_order: 3,
            restore_from_checkpoint: context.should_restore_stage(3),
            variables: &context.variables,
            manifest: visual_manifest,
            temperature: 0.3,
        })
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.chapter_id,
            context.project_id,
            "visual_anchor_extraction",
            3,
            context.task_id,
            &visual_trace,
        )
        .await?;
    let visual_out = visual_trace.response_text.clone();
    checkpoint_insert_trace(checkpoint, "visual_trace", &visual_trace)?;
    let visual_json = context
        .service
        .repair_and_validate_manga_llm_output(&visual_trace, |value| {
            validate_manga_visual_anchor_source_span_ids(value, &context.source_spans)?;
            Ok(value.clone())
        })
        .await?;
    context
        .service
        .save_or_validate_manga_llm_checkpoint(&visual_trace, &visual_json)
        .await?;
    checkpoint_insert(checkpoint, "visual_out", json!(visual_out.clone()));
    context
        .service
        .save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id: context.chapter_id,
            project_id: context.project_id,
            stage: "visual_anchor_extraction",
            current: 3,
            message: "已完成视觉锚点抽取",
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
            stage: "item_extraction",
            current: 4,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在抽取漫画关键物品",
            task_id: context.task_id,
            allow_cancel_override: false,
        })
        .await?;

    Ok((visual_trace, visual_out, visual_json))
}
