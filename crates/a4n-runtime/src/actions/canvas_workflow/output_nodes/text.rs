use super::super::execution::record_canvas_node_checkpoint;
use super::super::inputs::build_canvas_output_value;
use super::super::node_lifecycle::{
    CanvasRunHandle, finish_canvas_node_success, start_canvas_node_run,
};
use super::super::persistence::update_canvas_node_artifact;
use super::super::{CanvasGraphDocument, CanvasRunSummary};
use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasTextAssetRecordInput as UpsertCanvasTextAssetInput;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_COMPLETED;
use chrono::Utc;
use serde_json::json;

pub(in crate::actions::canvas_workflow) async fn run_canvas_text_output_node(
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
        "text",
        "TextOutputNode",
    )
    .await?;
    let text = output
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "TextOutputNode 缺少文本输入".to_string())?;
    let title = canvas_output_title(node, "文本输出");
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
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
    let asset = state
        .orchestrator
        .upsert_canvas_text_asset_record(
            project_id,
            UpsertCanvasTextAssetInput {
                id: node
                    .ref_type
                    .as_deref()
                    .is_some_and(|value| value == "canvas_text_asset")
                    .then_some(node.ref_id)
                    .flatten(),
                title: title.clone(),
                text_kind: "canvas_output".to_string(),
                content: text.to_string(),
            },
            Utc::now().naive_utc(),
        )
        .await
        .map_err(app_error_message)?;
    update_canvas_node_artifact(
        state,
        node.id,
        "canvas_text_asset",
        asset.id,
        json!({
            "data_type": "text",
            "canvas_text_asset_id": asset.id,
            "title": asset.title,
            "text": asset.content}),
    )
    .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(json!({
            "data_type": "text",
            "canvas_text_asset_id": asset.id,
            "title": asset.title,
            "text": asset.content})),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}

fn canvas_output_title(node: &canvas_node::Model, fallback: &str) -> String {
    node.config_json
        .get("title")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}
