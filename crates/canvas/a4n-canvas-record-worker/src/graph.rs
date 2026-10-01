use a4n_canvas_record_worker_contract::{
    CanvasGraphDocument, CanvasGraphProjection, CanvasGraphReadResult, ReadCanvasGraphInput,
};
use a4n_db::canvas::{
    CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput, CanvasNodeRecordInput,
};
use serde_json::Value;

use super::WorkerFailure;

pub(super) fn canvas_graph_document(
    document: CanvasGraphRecordDocument,
) -> Result<CanvasGraphDocument, WorkerFailure> {
    let graph = document.graph;
    let viewport_json = graph
        .viewport_json
        .map(|value| canvas_json_object(value, "canvas_graph.viewport_json"))
        .transpose()?;
    let nodes = document
        .nodes
        .into_iter()
        .map(|node| {
            Ok(a4n_canvas_record_worker_contract::CanvasNodeSummary {
                id: node.id,
                node_kind: node.node_kind,
                ref_type: node.ref_type,
                ref_id: node.ref_id,
                config_json: canvas_json_object(node.config_json, "canvas_node.config_json")?,
                position_json: canvas_json_object(node.position_json, "canvas_node.position_json")?,
                state: node.state,
            })
        })
        .collect::<Result<Vec<_>, WorkerFailure>>()?;
    let edges = document
        .edges
        .into_iter()
        .map(|edge| {
            Ok(a4n_canvas_record_worker_contract::CanvasEdgeSummary {
                id: edge.id,
                source_node_id: edge.source_node_id,
                target_node_id: edge.target_node_id,
                source_port: edge.source_port,
                target_port: edge.target_port,
                role: edge.role,
                config_json: canvas_json_object(edge.config_json, "canvas_edge.config_json")?,
            })
        })
        .collect::<Result<Vec<_>, WorkerFailure>>()?;
    Ok(CanvasGraphDocument {
        graph: a4n_canvas_record_worker_contract::CanvasGraphSummary {
            id: graph.id,
            name: graph.name,
            graph_kind: graph.graph_kind,
            version: graph.version,
            viewport_json,
        },
        nodes,
        edges,
    })
}

fn canvas_json_object(
    value: Value,
    field: &str,
) -> Result<a4n_canvas_record_worker_contract::CanvasJsonObject, WorkerFailure> {
    match value {
        Value::Object(object) => Ok(object.into_iter().collect()),
        _ => Err(WorkerFailure::internal(format!(
            "{field} 不是 JSON 对象，无法输出 CanvasGraphDocument"
        ))),
    }
}

pub(super) fn canvas_graph_record_input(
    input: a4n_canvas_record_worker_contract::CanvasGraphInput,
    expected_version: Option<i32>,
) -> CanvasGraphRecordInput {
    CanvasGraphRecordInput {
        id: input.id,
        expected_version,
        name: Some(input.name),
        graph_kind: Some(input.graph_kind),
        viewport_json: input.viewport_json.map(canvas_json_value),
        nodes: input
            .nodes
            .into_iter()
            .map(|node| CanvasNodeRecordInput {
                client_id: node.client_id,
                node_kind: node.node_kind,
                ref_type: node.ref_type,
                ref_id: node.ref_id,
                config_json: Some(canvas_json_value(node.config_json)),
                position_json: canvas_json_value(node.position_json),
                state: Some(node.state),
            })
            .collect(),
        edges: input
            .edges
            .into_iter()
            .map(|edge| CanvasEdgeRecordInput {
                source_client_id: edge.source_client_id,
                target_client_id: edge.target_client_id,
                source_port: edge.source_port,
                target_port: edge.target_port,
                role: edge.role,
                config_json: Some(canvas_json_value(edge.config_json)),
            })
            .collect(),
    }
}

fn canvas_json_value(input: a4n_canvas_record_worker_contract::CanvasJsonObject) -> Value {
    Value::Object(input.into_iter().collect())
}

pub(super) fn read_canvas_graph(
    graph: CanvasGraphDocument,
    input: ReadCanvasGraphInput,
) -> Result<CanvasGraphReadResult, WorkerFailure> {
    check_version(&graph, input.expected_version)?;
    let projection = match input.node_ids {
        Some(ids) => {
            if ids.is_empty() || ids.len() > 100 || input.offset != 0 || input.limit.is_some() {
                return Err(WorkerFailure::invalid_request(
                    "节点详情需要 1..100 个 ID，不能同时分页",
                ));
            }
            let ids = ids.into_iter().collect::<std::collections::HashSet<_>>();
            for id in &ids {
                if !graph.nodes.iter().any(|node| node.id == *id) {
                    return Err(WorkerFailure::not_found(format!(
                        "画布 {} 不存在节点 {id}",
                        input.graph_id
                    )));
                }
            }
            let edges = graph
                .edges
                .iter()
                .filter(|edge| {
                    ids.contains(&edge.source_node_id) || ids.contains(&edge.target_node_id)
                })
                .cloned()
                .collect::<Vec<_>>();
            let neighbors = edges
                .iter()
                .flat_map(|edge| [edge.source_node_id, edge.target_node_id])
                .collect::<std::collections::HashSet<_>>();
            CanvasGraphProjection::Nodes {
                nodes: graph
                    .nodes
                    .iter()
                    .filter(|node| ids.contains(&node.id))
                    .cloned()
                    .collect(),
                adjacent_nodes: graph
                    .nodes
                    .iter()
                    .filter(|node| neighbors.contains(&node.id) && !ids.contains(&node.id))
                    .map(node_index)
                    .collect(),
                edges,
            }
        }
        None => {
            let limit = input.limit.unwrap_or(20) as usize;
            let offset = input.offset as usize;
            if !(1..=100).contains(&limit) || offset > graph.nodes.len() {
                return Err(WorkerFailure::invalid_request(
                    "画布索引分页超出范围，limit 必须在 1..100",
                ));
            }
            let end = (offset + limit).min(graph.nodes.len());
            CanvasGraphProjection::Index {
                nodes: graph.nodes[offset..end].iter().map(node_index).collect(),
                total: graph.nodes.len(),
                next_offset: (end < graph.nodes.len()).then_some(end as u32),
            }
        }
    };
    Ok(CanvasGraphReadResult {
        resource: resource(&graph),
        projection,
    })
}

pub(super) fn apply_operation(
    graph: &mut CanvasGraphDocument,
    operation: a4n_canvas_record_worker_contract::CanvasGraphEditOperation,
) -> Result<(), WorkerFailure> {
    use a4n_canvas_record_worker_contract::CanvasGraphEditOperation;
    match operation {
        CanvasGraphEditOperation::SetNodeConfig {
            node_id,
            config_json,
        } => {
            graph
                .nodes
                .iter_mut()
                .find(|node| node.id == node_id)
                .ok_or_else(|| WorkerFailure::not_found(format!("画布不存在节点 {node_id}")))?
                .config_json = config_json;
        }
        CanvasGraphEditOperation::Disconnect { edge_id } => {
            let index = graph
                .edges
                .iter()
                .position(|edge| edge.id == edge_id)
                .ok_or_else(|| WorkerFailure::not_found(format!("画布不存在连线 {edge_id}")))?;
            graph.edges.remove(index);
        }
        CanvasGraphEditOperation::Connect { edge } => {
            let resolve = |id: &str| {
                graph
                    .nodes
                    .iter()
                    .find(|node| node.id.to_string() == id)
                    .map(|node| node.id)
                    .ok_or_else(|| WorkerFailure::not_found(format!("画布不存在节点 {id}")))
            };
            let connection = a4n_canvas_record_worker_contract::CanvasEdgeSummary {
                id: 0,
                source_node_id: resolve(&edge.source_client_id)?,
                target_node_id: resolve(&edge.target_client_id)?,
                source_port: edge.source_port,
                target_port: edge.target_port,
                role: edge.role,
                config_json: edge.config_json,
            };
            if graph.edges.iter().any(|existing| {
                existing.source_node_id == connection.source_node_id
                    && existing.target_node_id == connection.target_node_id
                    && existing.source_port == connection.source_port
                    && existing.target_port == connection.target_port
                    && existing.role == connection.role
            }) {
                return Err(WorkerFailure::invalid_request(
                    "连线已存在；修改前请显式断开原连线",
                ));
            }
            graph.edges.push(connection);
        }
    }
    Ok(())
}

pub(super) fn graph_input(
    graph: CanvasGraphDocument,
) -> a4n_canvas_record_worker_contract::CanvasGraphInput {
    a4n_canvas_record_worker_contract::CanvasGraphInput {
        id: Some(graph.graph.id),
        name: graph.graph.name,
        graph_kind: graph.graph.graph_kind,
        viewport_json: graph.graph.viewport_json,
        nodes: graph
            .nodes
            .into_iter()
            .map(|node| a4n_canvas_record_worker_contract::CanvasNodeInput {
                client_id: node.id.to_string(),
                node_kind: node.node_kind,
                ref_type: node.ref_type,
                ref_id: node.ref_id,
                config_json: node.config_json,
                position_json: node.position_json,
                state: node.state,
            })
            .collect(),
        edges: graph
            .edges
            .into_iter()
            .map(|edge| a4n_canvas_record_worker_contract::CanvasEdgeInput {
                source_client_id: edge.source_node_id.to_string(),
                target_client_id: edge.target_node_id.to_string(),
                source_port: edge.source_port,
                target_port: edge.target_port,
                role: edge.role,
                config_json: edge.config_json,
            })
            .collect(),
    }
}

fn node_index(
    node: &a4n_canvas_record_worker_contract::CanvasNodeSummary,
) -> a4n_canvas_record_worker_contract::CanvasNodeIndex {
    a4n_canvas_record_worker_contract::CanvasNodeIndex {
        id: node.id,
        node_kind: node.node_kind.clone(),
        state: node.state.clone(),
    }
}

pub(super) fn resource(graph: &CanvasGraphDocument) -> a4n_api_contract::agent::AgentResourceRef {
    a4n_api_contract::agent::AgentResourceRef {
        kind: "canvas_graph".into(),
        id: graph.graph.id.to_string(),
        label: Some(graph.graph.name.clone()),
        revision: Some(graph.graph.version.to_string()),
        hash: None,
    }
}

pub(super) fn check_version(
    graph: &CanvasGraphDocument,
    expected: Option<i32>,
) -> Result<(), WorkerFailure> {
    if expected.is_some_and(|version| version != graph.graph.version) {
        return Err(WorkerFailure::conflict(format!(
            "画布版本冲突：expected={expected:?}, current={}",
            graph.graph.version
        )));
    }
    Ok(())
}
