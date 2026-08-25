use std::collections::{BTreeMap, HashMap, HashSet};

use crate::canvas::CanvasDbError;

use super::super::types::{CanvasGraphRecordInput, CanvasNodeRecordInput};
use super::{
    common::{required_canvas_record_text, validate_canvas_record_viewport_json},
    dag::validate_canvas_graph_record_is_dag,
    edges::{
        canvas_record_edge_order_index, validate_canvas_record_contiguous_order_indexes,
        validate_canvas_record_edge_contract,
    },
    nodes::validate_canvas_record_node_ref_shape,
};

pub(in crate::canvas::graph_record) fn validate_canvas_graph_record_input(
    input: &CanvasGraphRecordInput,
) -> Result<(), CanvasDbError> {
    validate_canvas_record_viewport_json(
        input.viewport_json.as_ref(),
        "canvas_graph.viewport_json",
    )?;
    let mut client_ids = HashSet::new();
    let mut nodes_by_client_id = HashMap::<String, &CanvasNodeRecordInput>::new();
    let mut edge_keys = HashSet::new();
    let mut assemble_clip_orders = BTreeMap::<String, Vec<i64>>::new();

    for node in &input.nodes {
        let client_id = required_canvas_record_text(&node.client_id, "node.client_id")?;
        if !client_ids.insert(client_id.clone()) {
            return Err(CanvasDbError::InvalidArgument(format!(
                "节点 client_id 重复: {}",
                node.client_id
            )));
        }
        required_canvas_record_text(&node.node_kind, "node.node_kind")?;
        if !node.position_json.is_object() {
            return Err(CanvasDbError::InvalidArgument(
                "node.position_json 必须是 JSON 对象".to_string(),
            ));
        }
        validate_canvas_record_node_ref_shape(node)?;
        nodes_by_client_id.insert(client_id, node);
    }

    for edge in &input.edges {
        let source_client_id =
            required_canvas_record_text(&edge.source_client_id, "edge.source_client_id")?;
        let target_client_id =
            required_canvas_record_text(&edge.target_client_id, "edge.target_client_id")?;
        if !client_ids.contains(source_client_id.as_str()) {
            return Err(CanvasDbError::InvalidArgument(format!(
                "连线 source_client_id 不存在: {}",
                edge.source_client_id
            )));
        }
        if !client_ids.contains(target_client_id.as_str()) {
            return Err(CanvasDbError::InvalidArgument(format!(
                "连线 target_client_id 不存在: {}",
                edge.target_client_id
            )));
        }
        if source_client_id == target_client_id {
            return Err(CanvasDbError::InvalidArgument(format!(
                "画布不允许节点自连: {source_client_id}"
            )));
        }

        let source_port = required_canvas_record_text(&edge.source_port, "edge.source_port")?;
        let target_port = required_canvas_record_text(&edge.target_port, "edge.target_port")?;
        let role = required_canvas_record_text(&edge.role, "edge.role")?;
        if !edge_keys.insert((
            source_client_id.clone(),
            target_client_id.clone(),
            source_port.clone(),
            target_port.clone(),
            role.clone(),
        )) {
            return Err(CanvasDbError::InvalidArgument(
                "画布存在重复连线".to_string(),
            ));
        }

        let source = nodes_by_client_id
            .get(&source_client_id)
            .copied()
            .expect("source node was checked above");
        let target = nodes_by_client_id
            .get(&target_client_id)
            .copied()
            .expect("target node was checked above");
        validate_canvas_record_edge_contract(
            &source.node_kind,
            &source_port,
            &target.node_kind,
            &target_port,
            &role,
        )?;
        if target.node_kind == "AssembleVideoNode"
            && source_port == "video"
            && target_port == "clip"
            && role == "assembly_clip"
        {
            let order_index =
                canvas_record_edge_order_index(edge.config_json.as_ref(), "AssembleVideoNode")?;
            assemble_clip_orders
                .entry(target_client_id.clone())
                .or_default()
                .push(order_index);
        }
    }

    validate_canvas_record_contiguous_order_indexes(&assemble_clip_orders, "AssembleVideoNode")?;
    validate_canvas_graph_record_is_dag(input)?;
    Ok(())
}
