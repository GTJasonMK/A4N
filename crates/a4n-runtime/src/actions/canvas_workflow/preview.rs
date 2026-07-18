mod context;
mod contracts;
mod fallback;
mod image;
mod llm;
mod video;

pub use contracts::{CanvasNodeRequestPreview, PreviewCanvasNodeRequestInput};

use crate::AppState;
use context::load_canvas_preview_document;
use fallback::generic_canvas_node_request_preview;

pub async fn preview_canvas_node_request(
    state: &AppState,
    user_id: i64,
    input: PreviewCanvasNodeRequestInput,
) -> Result<CanvasNodeRequestPreview, String> {
    let document =
        load_canvas_preview_document(state, user_id, input.project_id, input.graph_id).await?;
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
        _ => Ok(generic_canvas_node_request_preview(&input, node)),
    }
}
