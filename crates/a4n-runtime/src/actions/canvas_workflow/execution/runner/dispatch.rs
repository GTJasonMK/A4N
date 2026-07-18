use super::super::graph::unsupported_canvas_node_execution_error;
use crate::actions::canvas_workflow::media_nodes::{
    run_canvas_assemble_node, run_canvas_extract_frame_node, run_canvas_generate_image_node,
    run_canvas_generate_video_node, run_canvas_trim_video_node,
};
use crate::actions::canvas_workflow::node_lifecycle::CanvasRunHandle;
use crate::actions::canvas_workflow::output_nodes::{
    run_canvas_audio_output_node, run_canvas_export_video_node, run_canvas_image_output_node,
    run_canvas_text_output_node,
};
use crate::actions::canvas_workflow::probe_node::run_canvas_probe_node;
use crate::actions::canvas_workflow::text_nodes::{
    run_canvas_llm_text_node, run_canvas_render_text_node,
};
use crate::actions::canvas_workflow::{CanvasGraphDocument, CanvasRunSummary};
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;

pub(super) async fn run_canvas_operation_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    match node.node_kind.as_str() {
        "RenderTextNode" => {
            run_canvas_render_text_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "LlmTextNode" => {
            run_canvas_llm_text_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "GenerateImageNode" => {
            run_canvas_generate_image_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "GenerateVideoNode" => {
            run_canvas_generate_video_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "TrimVideoNode" => {
            run_canvas_trim_video_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "ExtractFrameNode" => {
            run_canvas_extract_frame_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "AssembleVideoNode" => {
            run_canvas_assemble_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "TextOutputNode" => {
            run_canvas_text_output_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "ImageOutputNode" => {
            run_canvas_image_output_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "AudioOutputNode" => {
            run_canvas_audio_output_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "VideoOutputNode" => {
            run_canvas_export_video_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "TextProbeNode" | "ImageProbeNode" | "VideoProbeNode" | "AudioProbeNode" => {
            run_canvas_probe_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        other => Err(unsupported_canvas_node_execution_error(other)),
    }
}
