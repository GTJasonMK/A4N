use super::{ModelRequestPreview, PreviewCanvasNodeRequestInput};
use crate::AppState;
use crate::actions::image_generation::{GenerateCanvasImageInput, preview_canvas_image_request};
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasGraphRecordDocument, build_canvas_image_node_input_for_preview,
};

pub(super) async fn preview_canvas_image_node(
    state: &AppState,
    user_id: i64,
    input: &PreviewCanvasNodeRequestInput,
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
) -> Result<ModelRequestPreview, String> {
    let image_input = build_canvas_image_node_input_for_preview(node, document)?;
    let image_input = GenerateCanvasImageInput {
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: node.id,
        workflow_run_id: None,
        prompt: image_input.prompt,
        config_id: image_input.config_id,
        width: image_input.width,
        height: image_input.height,
        source_node_ids: image_input.source_node_ids,
    };
    preview_canvas_image_request(state, user_id, image_input).await
}
