use super::{ModelRequestPreview, PreviewCanvasNodeRequestInput};
use crate::AppState;
use crate::actions::video_generation::{GenerateCanvasVideoInput, preview_canvas_video_request};
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasGraphRecordDocument, build_canvas_video_node_input_for_preview,
};

pub(super) async fn preview_canvas_video_node(
    state: &AppState,
    user_id: i64,
    input: &PreviewCanvasNodeRequestInput,
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
) -> Result<ModelRequestPreview, String> {
    let video_input = build_canvas_video_node_input_for_preview(node, document)?;
    let video_input = GenerateCanvasVideoInput {
        user_id,
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: node.id,
        workflow_run_id: None,
        prompt: video_input.prompt,
        negative_prompt: video_input.negative_prompt,
        config_id: video_input.config_id,
        width: video_input.width,
        height: video_input.height,
        duration_secs: video_input.duration_secs,
        fps: video_input.fps,
        first_frame_image_id: video_input.first_frame_image_id,
        last_frame_image_id: video_input.last_frame_image_id,
        reference_image_ids: video_input.reference_image_ids,
        source_node_ids: video_input.source_node_ids,
    };
    preview_canvas_video_request(state, video_input).await
}
