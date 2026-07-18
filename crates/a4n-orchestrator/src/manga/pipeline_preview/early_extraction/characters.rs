use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::super::pipeline_json::required_object_field;
use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS, MangaContextManifestInput,
    MangaContextPreviousOutput, manga_character_binding_user_message,
    validate_manga_character_binding_source_span_ids,
};
use super::super::call::{build_manga_preview_call, push_blocked_after};
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_characters(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    events_out: &str,
    events_json: &Value,
    reference_assets: &Value,
) -> Result<Option<(String, Value)>, AppError> {
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_character_binding",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_character_binding",
            context.chapter_id
        ),
        call_order: 1,
        restore_from_checkpoint: context.should_restore_stage(1),
        manifest: MangaContextManifestInput {
            prompt_renderer: context.prompt_renderer,
            project_id: context.project_id,
            variables: &context.variables,
            chapter_content: &context.chapter_content,
            source_spans: &context.source_spans,
            prompt_names: &["shooting_script_projection", "manga_character_binding"],
            previous_outputs: &[MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
                prompt_name: "shooting_script_projection",
                parsed_output: &json!({ "events": events_json }),
                raw_output: Some(events_out),
            }],
            reference_assets: Some(reference_assets),
            dynamic_user_content: manga_character_binding_user_message(
                &context.chapter_content,
                events_json.clone(),
                &context.source_spans,
                reference_assets.clone(),
            )?,
        },
        temperature: 0.3,
    })
    .await?;
    let (characters_out, stage_json) = match call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_character_binding",
                "漫画 Step 1：角色绑定",
                "manga_character_binding",
            ));
            (serde_json::to_string(&parsed_output)?, parsed_output)
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_character_binding",
                "漫画 Step 1：角色绑定",
                "manga_character_binding",
                None,
                *request,
            ));
            push_blocked_after(steps, "manga_speech_selection");
            return Ok(None);
        }
    };
    validate_manga_character_binding_source_span_ids(&stage_json, &context.source_spans)?;
    let characters_json =
        required_object_field(&stage_json, "characters", "manga_character_binding")?;

    Ok(Some((characters_out, characters_json)))
}
