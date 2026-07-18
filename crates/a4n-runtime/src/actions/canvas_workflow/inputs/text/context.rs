use super::super::rules::incoming_edges;
use super::super::sources::{source_canvas_text_asset_id, source_text_value};
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(super) struct CanvasTextSourceContext {
    pub(super) text: String,
    pub(super) canvas_text_asset_id: Option<i64>,
    pub(super) source_node_ids: Vec<i64>,
}

pub(super) async fn collect_canvas_text_source_context(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    consumer_label: &str,
) -> Result<CanvasTextSourceContext, String> {
    let incoming = incoming_edges(document, node.id);
    let mut source_parts = Vec::new();
    let mut source_node_ids = Vec::new();
    let mut canvas_text_asset_id = source_canvas_text_asset_id(node);

    for edge in incoming {
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
        if let Some(text) =
            source_text_value(state, workflow_run_id, source, document, consumer_label).await?
        {
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
