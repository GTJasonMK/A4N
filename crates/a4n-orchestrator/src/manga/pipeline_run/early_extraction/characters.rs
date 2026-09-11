use a4n_error::AppError;
use serde_json::Value;

use super::super::super::MangaPipelineStage;
use super::super::super::pipeline_json::required_object_field;
use super::super::super::pipeline_prompt::{
    MangaRenderTraceInput, manga_character_binding_user_message,
    parse_and_validate_manga_llm_output, validate_manga_character_binding_source_span_ids,
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
        context.reference_assets.clone(),
    )?;
    let characters_trace = context
        .service
        .render_and_chat_trace(MangaRenderTraceInput {
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
        })
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
    let characters_stage_json = parse_and_validate_manga_llm_output(&characters_trace, |value| {
        validate_manga_character_binding_source_span_ids(value, &context.source_spans)?;
        Ok(value.clone())
    })?;
    context
        .service
        .save_or_validate_manga_llm_checkpoint(&characters_trace, &characters_stage_json)
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
