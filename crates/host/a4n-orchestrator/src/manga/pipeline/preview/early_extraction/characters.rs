use a4n_error::AppError;
use serde_json::Value;

use super::super::super::json::required_object_field;
use super::super::super::prompt::{
    manga_character_binding_user_message, validate_manga_character_binding_source_span_ids,
};
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_characters(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    events_json: &Value,
    reference_assets: &Value,
) -> Result<Option<Value>, AppError> {
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_character_binding",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_character_binding",
            context.chapter_id
        ),
        call_order: 1,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_character_binding_user_message(
            &context.chapter_content,
            events_json.clone(),
            &context.source_spans,
            &context.source_markers,
            reference_assets.clone(),
        )?,
        temperature: 0.3,
    })
    .await?;
    let stage_json = match call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_character_binding",
                "漫画 Step 1：角色绑定",
                "manga_character_binding",
            ));
            parsed_output
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_character_binding",
                "漫画 Step 1：角色绑定",
                "manga_character_binding",
                None,
                *request,
            ));
            return Ok(None);
        }
    };
    validate_manga_character_binding_source_span_ids(&stage_json, &context.source_spans)?;
    let characters_json =
        required_object_field(&stage_json, "characters", "manga_character_binding")?;

    Ok(Some(characters_json))
}
