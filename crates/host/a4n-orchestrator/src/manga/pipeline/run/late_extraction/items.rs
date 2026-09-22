use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::MangaPipelineStage;
use super::super::super::prompt::{
    MangaRenderTraceInput, manga_item_available_source_span_ids,
    manga_item_extraction_user_message, parse_and_validate_manga_llm_output,
    validate_manga_item_extraction_source_span_ids,
};
use super::super::contracts::MangaRunContext;

pub(super) async fn run_item_extraction_stage(
    context: &MangaRunContext<'_>,
    speech_json: &Value,
    visual_json: &Value,
) -> Result<Value, AppError> {
    let items_user = manga_item_extraction_user_message(speech_json.clone(), visual_json.clone())?;
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
            restore_from_checkpoint: context.resume_from_checkpoint,
            variables: &context.variables,
            user_content: items_user,
            temperature: 0.3,
        })
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.workflow_run_id,
            "item_extraction",
            4,
            context.task_id,
            &items_trace,
        )
        .await?;
    let allowed_source_span_ids = manga_item_available_source_span_ids(speech_json, visual_json)?;
    let items_json = parse_and_validate_manga_llm_output(&items_trace, |value| {
        validate_manga_item_extraction_source_span_ids(
            value,
            &context.source_spans,
            &allowed_source_span_ids,
        )?;
        Ok(value.clone())
    })?;
    context
        .service
        .save_or_validate_manga_llm_checkpoint(&items_trace, &items_json)
        .await?;
    context
        .advance_stage(MangaPipelineStage::ComicSynthesis)
        .await?;

    Ok(items_json)
}
