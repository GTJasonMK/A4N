use super::super::execution::record_canvas_node_checkpoint;
use super::super::inputs::build_canvas_output_value;
use super::super::node_lifecycle::{
    CanvasRunHandle, finish_canvas_node_success, start_canvas_node_run,
};
use super::super::persistence::update_canvas_node_artifact;
use super::super::{CanvasGraphDocument, CanvasRunSummary};
use crate::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_COMPLETED;
use serde_json::json;

pub(in crate::actions::canvas_workflow) async fn run_canvas_image_output_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let output = build_canvas_output_value(
        Some(state),
        upstream_run_id,
        node,
        document,
        "image",
        "ImageOutputNode",
    )
    .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "generated_image_id": output.generated_image_id,
        "media_asset_id": output.media_asset_id,
        "source_node_ids": output.source_node_ids});
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
    let (ref_type, ref_id, artifact_json) = if let Some(image_id) = output.generated_image_id {
        let image = state
            .orchestrator
            .generated_image_by_id(image_id)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| format!("图片输出不存在: {image_id}"))?;
        (
            "generated_image",
            image_id,
            json!({
                "data_type": "image",
                "generated_image_id": image_id,
                "file_path": image.file_path,
                "media_asset_id": output.media_asset_id}),
        )
    } else {
        let media_asset_id = output
            .media_asset_id
            .ok_or_else(|| "ImageOutputNode 缺少图片输入".to_string())?;
        let asset = state
            .orchestrator
            .canvas_media_asset_by_id(media_asset_id)
            .await
            .map_err(|err| err.to_string())?;
        (
            "media_asset",
            media_asset_id,
            json!({
                "data_type": "image",
                "media_asset_id": media_asset_id,
                "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone())}),
        )
    };
    update_canvas_node_artifact(state, node.id, ref_type, ref_id, artifact_json.clone()).await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(artifact_json),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}
