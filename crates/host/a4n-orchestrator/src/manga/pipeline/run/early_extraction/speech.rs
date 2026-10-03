use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::MangaPipelineStage;
use super::super::super::prompt::{
    MangaRenderTraceInput, manga_speech_selection_user_message,
    validate_manga_speech_selection_source_span_ids,
};
use super::super::contracts::MangaRunContext;

pub(super) struct SpeechSelectionStageInput<'a> {
    pub(super) events_json: &'a Value,
    pub(super) characters_json: &'a Value,
}

pub(super) async fn run_speech_selection_stage(
    context: &MangaRunContext<'_>,
    input: SpeechSelectionStageInput<'_>,
) -> Result<Value, AppError> {
    let SpeechSelectionStageInput {
        events_json,
        characters_json,
    } = input;
    let speech_user = manga_speech_selection_user_message(
        characters_json.clone(),
        events_json.clone(),
        &context.source_spans,
        &context.source_markers,
        context.reference_assets.clone(),
    )?;
    let (speech_trace, speech_json) = context
        .service
        .render_and_validate_manga_call_with_markers(
            MangaRenderTraceInput {
                prompt_renderer: context.prompt_renderer,
                prompt_name: "manga_speech_selection",
                project_id: context.project_id,
                workflow_run_id: context.workflow_run_id,
                call_key: format!(
                    "manga_pipeline:{}:manga_speech_selection",
                    context.chapter_id
                ),
                call_order: 2,
                restore_from_checkpoint: context.resume_from_checkpoint,
                variables: &context.variables,
                user_content: speech_user,
                temperature: 0.3,
                cancel_token: context.cancel_token,
                stream_id: "raw_json".to_string(),
                stream_reporter: context.stream_reporter,
            },
            &context.source_markers,
            |value| {
                validate_manga_speech_selection_source_span_ids(
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
            "speech_selection",
            2,
            context.task_id,
            &speech_trace,
        )
        .await?;
    context
        .advance_stage(MangaPipelineStage::VisualAnchorExtraction)
        .await?;

    Ok(speech_json)
}
