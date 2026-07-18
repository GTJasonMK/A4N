use super::super::super::inputs::{build_canvas_video_input_for_run, first_canvas_text_asset_id};
use super::super::super::node_lifecycle::{CanvasRunHandle, start_canvas_node_run};
use super::super::super::persistence::{
    FreeformCanvasSpecInput, upsert_freeform_canvas_generation_spec,
};
use super::super::super::{CanvasGraphDocument, CanvasRunSummary};
use super::context::CanvasVideoNodeContext;
use super::results::finish_canvas_video_node;
use crate::AppState;
use crate::actions::video_generation::generate_canvas_video;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_RUNNING;
use serde_json::json;
use tokio_util::sync::CancellationToken;

pub(in crate::actions::canvas_workflow) async fn run_canvas_generate_video_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let mut video_input = build_canvas_video_input_for_run(
        Some(state),
        upstream_run_id,
        user_id,
        project_id,
        graph_id,
        node,
        document,
    )
    .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let run_input_json = serde_json::to_value(&video_input).map_err(|err| err.to_string())?;
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_video",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "prompt": video_input.prompt,
                "first_frame_image_id": video_input.first_frame_image_id,
                "reference_image_ids": video_input.reference_image_ids,
                "source_node_ids": video_input.source_node_ids}),
            params_json: json!({
                "workflow": "generate_video",
                "config_id": video_input.config_id,
                "width": video_input.width,
                "height": video_input.height,
                "duration_secs": video_input.duration_secs,
                "fps": video_input.fps,
                "node_config": node.config_json}),
        },
    )
    .await?;
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        run_input_json,
        run_handle,
    )
    .await?;
    video_input.workflow_run_id = Some(run.workflow_run_id);
    let input_json = serde_json::to_value(&video_input).map_err(|err| err.to_string())?;
    let result = generate_canvas_video(state, video_input, None, CancellationToken::new()).await;

    finish_canvas_video_node(
        state,
        CanvasVideoNodeContext {
            project_id,
            graph_id,
            mode: mode.to_string(),
            step_key,
            input_json,
            run,
            generation_spec_id: generation_spec.map(|spec| spec.id),
        },
        result,
    )
    .await
}
