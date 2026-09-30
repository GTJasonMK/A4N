use crate::canvas::{
    canvas_config_i64, canvas_config_string, incoming_edges,
    source_checkpoint::source_operation_checkpoint_output, source_node,
};
use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_db_models::models::generation::workflow_checkpoint;
use a4n_domain::CanvasNodeRefKind;
use a4n_domain::canvas::CanvasNodeRecord;
use serde_json::Value;

pub(crate) struct CanvasTextSourceContext {
    pub(crate) text: String,
    pub(crate) canvas_text_asset_id: Option<i64>,
    pub(crate) source_node_ids: Vec<i64>,
}

pub(crate) fn collect_canvas_text_source_context(
    node: &CanvasNodeRecord,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
    consumer_label: &str,
) -> Result<CanvasTextSourceContext, String> {
    let mut source_parts = Vec::new();
    let mut source_node_ids = Vec::new();
    let mut canvas_text_asset_id = source_canvas_text_asset_id(node)?;

    for edge in incoming_edges(document, node.id) {
        let source = source_node(document, edge.source_node_id)?;
        source_node_ids.push(source.id);
        if let Some(asset_id) = source_canvas_text_asset_id(source)?
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
    node: &CanvasNodeRecord,
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
    if let Some(asset_id) = source_canvas_text_asset_id(node)?
        && let Some(asset) = document
            .text_assets
            .iter()
            .find(|candidate| candidate.id == asset_id)
    {
        return Ok(Some(asset.content.clone()));
    }
    Ok(canvas_config_string(&node.config_json, "content")
        .or_else(|| canvas_config_string(&node.config_json, "text"))
        .or_else(|| canvas_config_string(&node.config_json, "observed_text")))
}

pub(crate) fn first_canvas_text_asset_id(
    node: &CanvasNodeRecord,
    document: &CanvasGraphRecordDocument,
) -> Result<Option<i64>, String> {
    if let Some(id) = source_canvas_text_asset_id(node)? {
        return Ok(Some(id));
    }
    for edge in incoming_edges(document, node.id) {
        if let Some(id) = source_canvas_text_asset_id(source_node(document, edge.source_node_id)?)?
        {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

pub(crate) fn source_canvas_text_asset_id(node: &CanvasNodeRecord) -> Result<Option<i64>, String> {
    if let Some(id) = node
        .ref_type
        .as_deref()
        .filter(|value| *value == CanvasNodeRefKind::CanvasTextAsset.as_str())
        .and(node.ref_id)
    {
        return Ok(Some(id));
    }
    canvas_config_i64(&node.config_json, "canvas_text_asset_id")
}
