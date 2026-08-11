use super::contracts::PreviewGalgamePipelineInput;
use crate::AppState;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};

pub async fn preview_galgame_pipeline_request(
    state: &AppState,
    user_id: i64,
    input: PreviewGalgamePipelineInput,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let prepared = state
        .orchestrator()
        .prepare_galgame_pipeline_preview_for_user_validated(
            user_id,
            input.project_id,
            input.start_from_stage,
            input.resume_from_checkpoint,
            prompt_renderer,
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut steps = Vec::with_capacity(prepared.len());
    for item in prepared {
        steps.push(
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: item.key,
                    title: item.title,
                    prompt_name: Some(item.prompt_name),
                    state: "ready".to_string(),
                    note: None,
                    request: item.request,
                },
            )
            .await?,
        );
    }
    Ok(model_request_preview(
        "galgame_pipeline",
        "Galgame 七阶段管线请求预览",
        steps,
    ))
}
