use super::{ModelRequestPreview, PreviewCanvasNodeRequestInput};
use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, chat_request_step_for_state, model_request_preview,
    },
};
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasGraphRecordDocument as CanvasGraphDocument, build_canvas_llm_text_request_for_preview,
};

pub(super) async fn preview_canvas_llm_node(
    state: &AppState,
    input: &PreviewCanvasNodeRequestInput,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<ModelRequestPreview, String> {
    let request = build_canvas_llm_text_request_for_preview(
        input.project_id,
        input.graph_id,
        node,
        document,
    )?;
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
