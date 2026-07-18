use super::checkpoint::source_operation_checkpoint_output;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use serde_json::Value;

pub(in crate::actions::canvas_workflow::inputs) async fn source_generated_image_id(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<i64>, String> {
    if let Some(value) =
        source_operation_checkpoint_output(state, workflow_run_id, node, consumer_label).await?
    {
        return Ok(value.get("generated_image_id").and_then(Value::as_i64));
    }
    if matches!(
        node.ref_type.as_deref(),
        Some("generated_image" | "legacy_generated_image")
    ) {
        return Ok(node.ref_id);
    }
    Ok(None)
}

pub(in crate::actions::canvas_workflow::inputs) async fn source_generated_video_id(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<i64>, String> {
    if let Some(value) =
        source_operation_checkpoint_output(state, workflow_run_id, node, consumer_label).await?
    {
        return Ok(value.get("generated_video_id").and_then(Value::as_i64));
    }
    if matches!(
        node.ref_type.as_deref(),
        Some("generated_video" | "legacy_generated_video")
    ) {
        return Ok(node.ref_id);
    }
    Ok(None)
}

pub(in crate::actions::canvas_workflow::inputs) async fn source_media_asset_id(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<i64>, String> {
    if let Some(value) =
        source_operation_checkpoint_output(state, workflow_run_id, node, consumer_label).await?
    {
        return Ok(value.get("media_asset_id").and_then(Value::as_i64));
    }
    if node.ref_type.as_deref() == Some("media_asset") {
        return Ok(node.ref_id);
    }
    Ok(None)
}
