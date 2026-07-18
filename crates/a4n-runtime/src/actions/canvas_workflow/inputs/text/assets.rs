use super::super::rules::incoming_edges;
use super::super::sources::source_canvas_text_asset_id;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) fn first_canvas_text_asset_id(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Option<i64> {
    source_canvas_text_asset_id(node).or_else(|| {
        incoming_edges(document, node.id)
            .into_iter()
            .find_map(|edge| {
                document
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == edge.source_node_id)
                    .and_then(source_canvas_text_asset_id)
            })
    })
}
