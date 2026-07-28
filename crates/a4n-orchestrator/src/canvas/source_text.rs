use crate::canvas::{
    canvas_config_i64, canvas_config_string, incoming_edges,
    source_checkpoint::source_operation_checkpoint_output,
};
use a4n_core::models::{canvas::canvas_node, generation::workflow_checkpoint};
use a4n_db::canvas::CanvasGraphRecordDocument;
use serde_json::Value;

pub(crate) struct CanvasTextSourceContext {
    pub(crate) text: String,
    pub(crate) canvas_text_asset_id: Option<i64>,
    pub(crate) source_node_ids: Vec<i64>,
}

pub(crate) fn collect_canvas_text_source_context(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
    consumer_label: &str,
) -> Result<CanvasTextSourceContext, String> {
    let mut source_parts = Vec::new();
    let mut source_node_ids = Vec::new();
    let mut canvas_text_asset_id = source_canvas_text_asset_id(node);

    for edge in incoming_edges(document, node.id) {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(asset_id) = source_canvas_text_asset_id(source)
            && canvas_text_asset_id.is_none()
        {
            canvas_text_asset_id = Some(asset_id);
        }
        if let Some(text) = source_text_value(source, document, checkpoints, consumer_label)? {
            source_parts.push(text);
        }
    }

    let text = source_parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(CanvasTextSourceContext {
        text,
        canvas_text_asset_id,
        source_node_ids,
    })
}

pub(crate) fn source_text_value(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
    consumer_label: &str,
) -> Result<Option<String>, String> {
    if let Some(value) = source_operation_checkpoint_output(checkpoints, node, consumer_label)? {
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

pub(crate) fn source_canvas_text_asset_id(node: &canvas_node::Model) -> Option<i64> {
    node.ref_type
        .as_deref()
        .is_some_and(|value| value == "canvas_text_asset")
        .then_some(node.ref_id)
        .flatten()
        .or_else(|| canvas_config_i64(&node.config_json, "canvas_text_asset_id"))
}
