use super::contracts::{CanvasNodeRequestPreview, PreviewCanvasNodeRequestInput};
use super::fallback::blocked_canvas_node_request_preview;
use crate::AppState;
use crate::actions::image_generation::{GenerateCanvasImageInput, preview_canvas_image_request};
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasGraphRecordDocument as CanvasGraphDocument, build_canvas_image_node_input_for_preview,
};

pub(super) async fn preview_canvas_image_node(
    state: &AppState,
    user_id: i64,
    input: &PreviewCanvasNodeRequestInput,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasNodeRequestPreview, String> {
    let image_input = match build_canvas_image_node_input_for_preview(node, document) {
        Ok(image_input) => GenerateCanvasImageInput {
            project_id: input.project_id,
            graph_id: input.graph_id,
            node_id: node.id,
            workflow_run_id: None,
            prompt: image_input.prompt,
            config_id: image_input.config_id,
            source_node_ids: image_input.source_node_ids,
        },
        Err(reason) => {
            return Ok(blocked_canvas_node_request_preview(
                input, node, document, reason,
            ));
        }
    };
    let preview = match preview_canvas_image_request(state, user_id, image_input).await {
        Ok(preview) => preview,
        Err(reason) => {
            return Ok(blocked_canvas_node_request_preview(
                input, node, document, reason,
            ));
        }
    };
    Ok(CanvasNodeRequestPreview {
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: input.node_id,
        node_kind: node.node_kind.clone(),
        status: "preview".to_string(),
        note: "已构造 GenerateImageNode 的图片 provider 请求预览；预览不会调用外部模型。"
            .to_string(),
        request: serde_json::to_value(preview).map_err(|err| err.to_string())?,
    })
}
