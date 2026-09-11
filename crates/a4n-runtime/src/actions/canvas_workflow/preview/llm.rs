use super::ModelRequestPreview;
use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, chat_request_step_for_state, model_request_preview,
    },
};
use a4n_llm::provider::ChatRequest;

pub(super) async fn preview_canvas_llm_node(
    state: &AppState,
    request: ChatRequest,
) -> Result<ModelRequestPreview, String> {
    Ok(model_request_preview(
        "canvas_llm_text_generation",
        "画布 LLM 文本模型请求预览",
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "canvas_llm_text".to_string(),
                    title: "画布 LLM provider 请求".to_string(),
                    prompt_name: None,
                    state: "ready".to_string(),
                    note: None,
                    request,
                },
            )
            .await?,
        ],
    ))
}
