use super::super::rules::{canvas_config_i64, canvas_config_string};
use super::checkpoint::source_operation_checkpoint_output;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;
use serde_json::Value;

pub(in crate::actions::canvas_workflow::inputs) async fn source_text_value(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    consumer_label: &str,
) -> Result<Option<String>, String> {
    if let Some(value) =
        source_operation_checkpoint_output(state, workflow_run_id, node, consumer_label).await?
    {
        return Ok(value
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string));
    }
    if let Some(asset_id) = source_canvas_text_asset_id(node)
        && let Some(asset) = document
            .text_assets
            .iter()
            .find(|candidate| candidate.id == asset_id)
    {
        return Ok(Some(asset.content.clone()));
    }
    Ok(canvas_config_string(
        &node.config_json,
        &[
            "text",
            "content",
            "prompt_text",
            "prompt",
            "title",
            "review_note",
        ],
    ))
}

pub(in crate::actions::canvas_workflow::inputs) fn source_canvas_text_asset_id(
    node: &canvas_node::Model,
) -> Option<i64> {
    node.ref_type
        .as_deref()
        .is_some_and(|value| value == "canvas_text_asset")
        .then_some(node.ref_id)
        .flatten()
        .or_else(|| canvas_config_i64(&node.config_json, "canvas_text_asset_id"))
}
