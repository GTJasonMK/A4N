use super::super::execution::record_canvas_node_checkpoint;
use super::super::inputs::build_canvas_export_video_input_for_run;
use super::super::node_lifecycle::{
    CanvasRunHandle, finish_canvas_node_success, start_canvas_node_run,
};
use super::super::persistence::update_canvas_node_artifact;
use super::super::{CanvasGraphDocument, CanvasRunSummary};
use crate::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_COMPLETED;
use serde_json::json;

pub(in crate::actions::canvas_workflow) async fn run_canvas_export_video_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let export_input =
        build_canvas_export_video_input_for_run(Some(state), upstream_run_id, node, document)
            .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "video_id": export_input.video_id,
        "source_node_ids": export_input.source_node_ids});
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    let video_id = export_input
        .video_id
        .ok_or_else(|| "VideoOutputNode 缺少可输出的视频输入".to_string())?;
    let video = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布输出视频不存在: {video_id}"))?;
    let output_json = json!({
        "data_type": "video",
        "generated_video_id": video_id,
        "file_path": video.file_path,
        "thumbnail_path": video.thumbnail_path,
        "output": {
            "kind": "video",
            "generated_video_id": video_id,
            "file_path": video.file_path,
            "thumbnail_path": video.thumbnail_path
        }
    });
    update_canvas_node_artifact(
        state,
        node.id,
        "generated_video",
        video_id,
        output_json.clone(),
    )
    .await?;

    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(output_json),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}
