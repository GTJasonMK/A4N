mod context;
mod image;
mod llm;
mod video;

use crate::actions::request_preview::ModelRequestPreview;
use a4n_api_contract::canvas::PreviewCanvasNodeRequestInput;

use crate::{AppState, actions::access};
use context::load_canvas_preview_document;

pub async fn preview_canvas_node_request(
    state: &AppState,
    user_id: i64,
    input: PreviewCanvasNodeRequestInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    let canvas = state.orchestrator();
    let document = load_canvas_preview_document(canvas, input.project_id, input.graph_id).await?;
    let node = document
        .nodes
        .iter()
        .find(|node| node.id == input.node_id)
        .ok_or_else(|| format!("画布节点不存在: {}", input.node_id))?;
    match node.node_kind.as_str() {
        "GenerateVideoNode" => {
            video::preview_canvas_video_node(state, user_id, &input, node, &document).await
        }
        "GenerateImageNode" => {
            image::preview_canvas_image_node(state, user_id, &input, node, &document).await
        }
        "LlmTextNode" => llm::preview_canvas_llm_node(state, &input, node, &document).await,
        _ => Err(format!("画布节点不支持模型请求预览: {}", node.node_kind)),
    }
}
