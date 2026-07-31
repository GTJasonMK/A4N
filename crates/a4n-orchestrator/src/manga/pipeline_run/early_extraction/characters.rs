use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::MangaPipelineStartStage;
use super::super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::super::pipeline_json::required_object_field;
use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS, MangaContextManifestInput,
    MangaContextPreviousOutput, MangaPromptGenerationTrace, MangaRenderTraceInput,
    build_manga_context_manifest, manga_character_binding_user_message,
    validate_manga_character_binding_source_span_ids,
};
use super::super::contracts::MangaRunContext;

pub(super) async fn run_character_binding_stage(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
    events_json: &Value,
    events_out: &str,
) -> Result<(MangaPromptGenerationTrace, String, Value), AppError> {
    let characters_user = manga_character_binding_user_message(
        &context.chapter_content,
        events_json.clone(),
        &context.source_spans,
        context.reference_assets.clone(),
    )?;
    let events_context = json!({ "events": events_json });
    let characters_manifest = build_manga_context_manifest(MangaContextManifestInput {
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        chapter_content: &context.chapter_content,
        source_spans: &context.source_spans,
        prompt_names: &["manga_character_binding"],
        previous_outputs: &[MangaContextPreviousOutput {
            pack_kind: MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
            prompt_name: "shooting_script_projection",
            parsed_output: &events_context,
            raw_output: Some(events_out),
        }],
        reference_assets: Some(&context.reference_assets),
        dynamic_user_content: characters_user,
    })
    .await?;
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
            restore_from_checkpoint: context.should_restore_stage(1),
            variables: &context.variables,
            manifest: characters_manifest,
            temperature: 0.3,
        })
        .await?;
    context
        .service
        .report_pipeline_llm_usage(
            context.chapter_id,
            context.project_id,
            "character_binding",
            1,
            context.task_id,
            &characters_trace,
        )
        .await?;
    let characters_out = characters_trace.response_text.clone();
    checkpoint_insert_trace(checkpoint, "characters_trace", &characters_trace)?;
    let characters_stage_json = context
        .service
        .repair_and_validate_manga_llm_output(&characters_trace, |value| {
            validate_manga_character_binding_source_span_ids(value, &context.source_spans)?;
            Ok(value.clone())
        })
        .await?;
    context
        .service
        .save_or_validate_manga_llm_checkpoint(&characters_trace, &characters_stage_json)
        .await?;
    let characters_json = required_object_field(
        &characters_stage_json,
        "characters",
        "manga_character_binding",
    )?;
    checkpoint_insert(checkpoint, "characters_out", json!(characters_out.clone()));
    checkpoint_insert(checkpoint, "characters", characters_json.clone());
    context
        .checkpoint_and_advance_stage(
            checkpoint,
            MangaPipelineStartStage::CharacterBinding,
            "已完成角色绑定",
            MangaPipelineStartStage::SpeechSelection,
        )
        .await?;

    Ok((characters_trace, characters_out, characters_json))
}
