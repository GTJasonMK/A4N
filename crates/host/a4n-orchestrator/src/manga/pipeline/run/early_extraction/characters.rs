use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::MangaPipelineStage;
use super::super::super::json::required_object_field;
use super::super::super::prompt::{
    MangaRenderTraceInput, manga_character_binding_user_message,
    validate_manga_character_binding_source_span_ids,
};
use super::super::contracts::MangaRunContext;

pub(super) async fn run_character_binding_stage(
    context: &MangaRunContext<'_>,
    events_json: &Value,
) -> Result<Value, AppError> {
    let characters_user = manga_character_binding_user_message(
        &context.chapter_content,
        events_json.clone(),
        &context.source_spans,
        &context.source_markers,
        context.reference_assets.clone(),
    )?;
    let (characters_trace, characters_stage_json) = context
        .service
        .render_and_validate_manga_call_with_markers(
            MangaRenderTraceInput {
                prompt_renderer: context.prompt_renderer,
                prompt_name: "manga_character_binding",
                project_id: context.project_id,
                workflow_run_id: context.workflow_run_id,
                call_key: format!(
                    "manga_pipeline:{}:manga_character_binding",
                    context.chapter_id
                ),
                call_order: 1,
                restore_from_checkpoint: context.resume_from_checkpoint,
                variables: &context.variables,
                user_content: characters_user,
                temperature: 0.3,
                cancel_token: context.cancel_token,
                stream_id: "raw_json".to_string(),
                stream_reporter: context.stream_reporter,
            },
            &context.source_markers,
            |value| {
                validate_manga_character_binding_source_span_ids(value, &context.source_spans)?;
                Ok(value.clone())
            },
        )
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.workflow_run_id,
            "character_binding",
            1,
            context.task_id,
            &characters_trace,
        )
        .await?;
    let characters_json = required_object_field(
        &characters_stage_json,
        "characters",
        "manga_character_binding",
    )?;
    context
        .advance_stage(MangaPipelineStage::SpeechSelection)
        .await?;

    Ok(characters_json)
}
