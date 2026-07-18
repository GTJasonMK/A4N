use super::kinds::canvas_node_kind_is_executable;
use super::topology::{topological_canvas_node_ids, upstream_canvas_node_ids};
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) fn runnable_canvas_node_ids(
    document: &CanvasGraphDocument,
    target_node_id: Option<i64>,
) -> Result<Vec<i64>, String> {
    let order = topological_canvas_node_ids(document)?;
    let allowed = match target_node_id {
        Some(node_id) => upstream_canvas_node_ids(document, node_id)?,
        None => document.nodes.iter().map(|node| node.id).collect(),
    };
    Ok(order
        .into_iter()
        .filter(|node_id| allowed.contains(node_id))
        .filter(|node_id| {
            document
                .nodes
                .iter()
                .find(|node| node.id == *node_id)
                .is_some_and(is_runnable_canvas_node)
        })
        .collect())
}

pub(in crate::actions::canvas_workflow::execution) fn is_runnable_canvas_node(
    node: &canvas_node::Model,
) -> bool {
    canvas_node_kind_is_executable(&node.node_kind)
}
