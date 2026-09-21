use std::collections::BTreeMap;

use crate::canvas::CanvasDbError;

use super::{
    contract::validate_canvas_record_edge_contract,
    order::{canvas_record_edge_order_index, validate_canvas_record_contiguous_order_indexes},
};
use crate::canvas::graph_record::types::CanvasGraphRecordDocument;

pub fn validate_canvas_record_document_edge_contract(
    document: &CanvasGraphRecordDocument,
) -> Result<(), CanvasDbError> {
    let nodes_by_id = document
        .nodes
        .iter()
        .map(|node| (node.id, node))
        .collect::<BTreeMap<_, _>>();
    let mut assemble_clip_orders = BTreeMap::<i64, Vec<i64>>::new();

    for edge in &document.edges {
        let source = nodes_by_id.get(&edge.source_node_id).ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!(
                "画布连线引用不存在的源节点: {}",
                edge.source_node_id
            ))
        })?;
        let target = nodes_by_id.get(&edge.target_node_id).ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!(
                "画布连线引用不存在的目标节点: {}",
                edge.target_node_id
            ))
        })?;
        validate_canvas_record_edge_contract(
            &source.node_kind,
            &edge.source_port,
            &target.node_kind,
            &edge.target_port,
            &edge.role,
        )?;
        if target.node_kind == "AssembleVideoNode"
            && edge.source_port == "video"
            && edge.target_port == "clip"
            && edge.role == "assembly_clip"
        {
            let order_index =
                canvas_record_edge_order_index(Some(&edge.config_json), "AssembleVideoNode")?;
            assemble_clip_orders
                .entry(edge.target_node_id)
                .or_default()
                .push(order_index);
        }
    }

    validate_canvas_record_contiguous_order_indexes(&assemble_clip_orders, "AssembleVideoNode")
}
