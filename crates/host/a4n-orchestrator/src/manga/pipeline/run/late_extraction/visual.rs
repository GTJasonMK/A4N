use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::MangaPipelineStage;
use super::super::super::prompt::{
    MangaRenderTraceInput, manga_visual_anchor_extraction_user_message,
    validate_manga_visual_anchor_source_span_ids,
};
use super::super::contracts::MangaRunContext;

pub(super) async fn run_visual_anchor_stage(
    context: &MangaRunContext<'_>,
    events_json: &Value,
) -> Result<Value, AppError> {
    let visual_user =
        manga_visual_anchor_extraction_user_message(events_json.clone(), &context.source_spans)?;
    let (visual_trace, visual_json) = context
        .service
        .render_and_validate_manga_call(
            MangaRenderTraceInput {
                prompt_renderer: context.prompt_renderer,
                prompt_name: "manga_visual_anchor_extraction",
                project_id: context.project_id,
                workflow_run_id: context.workflow_run_id,
                call_key: format!(
                    "manga_pipeline:{}:manga_visual_anchor_extraction",
                    context.chapter_id
                ),
                call_order: 3,
                restore_from_checkpoint: context.resume_from_checkpoint,
                variables: &context.variables,
                user_content: visual_user,
                temperature: 0.3,
                cancel_token: context.cancel_token,
                stream_id: "raw_json".to_string(),
                stream_reporter: context.stream_reporter,
            },
            |value| {
                validate_manga_visual_anchor_source_span_ids(
                    value,
                    &context.source_spans,
                    &crate::manga::adaptation_projection::manga_beat_refs(&context.adaptation.plan),
                )?;
                Ok(value.clone())
            },
        )
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.workflow_run_id,
            "visual_anchor_extraction",
            3,
            context.task_id,
            &visual_trace,
        )
        .await?;
    context
        .advance_stage(MangaPipelineStage::ItemExtraction)
        .await?;

    Ok(visual_json)
}
