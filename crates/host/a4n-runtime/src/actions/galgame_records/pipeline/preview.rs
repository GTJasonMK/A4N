use crate::AppState;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use a4n_api_contract::galgame::PreviewGalgamePipelineInput;

pub async fn preview_galgame_pipeline_request(
    state: &AppState,
    user_id: i64,
    input: PreviewGalgamePipelineInput,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let prepared = state
        .galgame_pipeline_preview_application()
        .preview(user_id, input, prompt_renderer)
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut steps = Vec::with_capacity(prepared.len());
    for item in prepared {
        let request = item
            .request
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| format!("Galgame 预览请求格式无效: {error}"))?
            .ok_or_else(|| "Galgame 预览请求缺少 LLM 请求体".to_string())?;
        steps.push(
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: item.key,
                    title: item.title,
                    prompt_name: Some(item.prompt_name),
                    prompt_assembly: None,
                    state: "ready".to_string(),
                    note: None,
                    request,
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
