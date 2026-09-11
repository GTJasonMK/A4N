use a4n_api_contract::{agent::AgentResourceRef, canvas::*};
use a4n_error::AppError;
use chrono::Utc;
use std::collections::HashSet;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn read_canvas_graph(
        &self,
        input: ReadCanvasGraphInput,
    ) -> Result<CanvasGraphReadResult, AppError> {
        if input.project_id <= 0
            || input.graph_id <= 0
            || input.expected_version.is_some_and(|v| v <= 0)
        {
            return Err(AppError::InvalidArgument(
                "项目、画布和版本必须是正整数".into(),
            ));
        }
        let graph = self
            .get_or_create_canvas_graph(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        check_version(&graph, input.expected_version)?;
        let projection = match input.node_ids {
            Some(ids) => {
                if ids.is_empty() || ids.len() > 100 || input.offset != 0 || input.limit.is_some() {
                    return Err(AppError::InvalidArgument(
                        "节点详情需要 1..100 个 ID，不能同时分页".into(),
                    ));
                }
                let ids: HashSet<_> = ids.into_iter().collect();
                for id in &ids {
                    if !graph.nodes.iter().any(|n| n.id == *id) {
                        return Err(AppError::NotFound(format!(
                            "画布 {} 不存在节点 {id}",
                            input.graph_id
                        )));
                    }
                }
                let edges: Vec<_> = graph
                    .edges
                    .iter()
                    .filter(|e| ids.contains(&e.source_node_id) || ids.contains(&e.target_node_id))
                    .cloned()
                    .collect();
                let neighbors: HashSet<_> = edges
                    .iter()
                    .flat_map(|e| [e.source_node_id, e.target_node_id])
                    .collect();
                CanvasGraphProjection::Nodes {
                    nodes: graph
                        .nodes
                        .iter()
                        .filter(|n| ids.contains(&n.id))
                        .cloned()
                        .collect(),
                    adjacent_nodes: graph
                        .nodes
                        .iter()
                        .filter(|n| neighbors.contains(&n.id) && !ids.contains(&n.id))
                        .map(node_index)
                        .collect(),
                    edges,
                }
            }
            None => {
                let limit = input.limit.unwrap_or(20) as usize;
                let offset = input.offset as usize;
                if !(1..=100).contains(&limit) || offset > graph.nodes.len() {
                    return Err(AppError::InvalidArgument(
                        "画布索引分页超出范围，limit 必须在 1..100".into(),
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

    pub async fn edit_canvas_graph(
        &self,
        input: EditCanvasGraphInput,
    ) -> Result<CanvasGraphEditResult, AppError> {
        if input.project_id <= 0
            || input.graph_id <= 0
            || input.expected_version <= 0
            || !(1..=20).contains(&input.operations.len())
        {
            return Err(AppError::InvalidArgument(
                "画布局部修改需要有效项目、画布、版本和 1..20 个操作".into(),
            ));
        }
        let original = self
            .get_or_create_canvas_graph(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        check_version(&original, Some(input.expected_version))?;
        let mut edited = original.clone();
        for operation in input.operations {
            apply_operation(&mut edited, operation)?;
        }
        let changed = edited != original;
        let saved = if changed {
            self.save_canvas_graph(
                input.project_id,
                Some(input.expected_version),
                graph_input(edited),
                Utc::now().naive_utc(),
            )
            .await?
        } else {
            original.clone()
        };
        Ok(CanvasGraphEditResult {
            resource: resource(&saved),
            changed,
            changed_nodes: saved
                .nodes
                .iter()
                .filter(|node| !original.nodes.contains(node))
                .cloned()
                .collect(),
            added_edges: saved
                .edges
                .iter()
                .filter(|edge| !original.edges.contains(edge))
                .cloned()
                .collect(),
            removed_edge_ids: original
                .edges
                .iter()
                .filter(|edge| !saved.edges.iter().any(|e| e.id == edge.id))
                .map(|e| e.id)
                .collect(),
        })
    }
}

fn apply_operation(
    graph: &mut CanvasGraphDocument,
    operation: CanvasGraphEditOperation,
) -> Result<(), AppError> {
    match operation {
        CanvasGraphEditOperation::SetNodeConfig {
            node_id,
            config_json,
        } => {
            graph
                .nodes
                .iter_mut()
                .find(|n| n.id == node_id)
                .ok_or_else(|| AppError::NotFound(format!("画布不存在节点 {node_id}")))?
                .config_json = config_json;
        }
        CanvasGraphEditOperation::Disconnect { edge_id } => {
            let index = graph
                .edges
                .iter()
                .position(|e| e.id == edge_id)
                .ok_or_else(|| AppError::NotFound(format!("画布不存在连线 {edge_id}")))?;
            graph.edges.remove(index);
        }
        CanvasGraphEditOperation::Connect { edge } => {
            let resolve = |id: &str| {
                graph
                    .nodes
                    .iter()
                    .find(|n| n.id.to_string() == id)
                    .map(|n| n.id)
                    .ok_or_else(|| AppError::NotFound(format!("画布不存在节点 {id}")))
            };
            let connection = CanvasEdgeSummary {
                id: 0,
                source_node_id: resolve(&edge.source_client_id)?,
                target_node_id: resolve(&edge.target_client_id)?,
                source_port: edge.source_port,
                target_port: edge.target_port,
                role: edge.role,
                config_json: edge.config_json,
            };
            if graph.edges.iter().any(|e| {
                e.source_node_id == connection.source_node_id
                    && e.target_node_id == connection.target_node_id
                    && e.source_port == connection.source_port
                    && e.target_port == connection.target_port
                    && e.role == connection.role
            }) {
                return Err(AppError::InvalidArgument(
                    "连线已存在；修改前请显式断开原连线".into(),
                ));
            }
            graph.edges.push(connection);
        }
    }
    Ok(())
}

fn node_index(node: &CanvasNodeSummary) -> CanvasNodeIndex {
    CanvasNodeIndex {
        id: node.id,
        node_kind: node.node_kind.clone(),
        state: node.state.clone(),
    }
}

fn resource(graph: &CanvasGraphDocument) -> AgentResourceRef {
    AgentResourceRef {
        kind: "canvas_graph".into(),
        id: graph.graph.id.to_string(),
        label: Some(graph.graph.name.clone()),
        revision: Some(graph.graph.version.to_string()),
        hash: None,
    }
}

fn check_version(graph: &CanvasGraphDocument, expected: Option<i32>) -> Result<(), AppError> {
    if expected.is_some_and(|v| v != graph.graph.version) {
        return Err(AppError::InvalidState {
            current: format!(
                "画布版本冲突：expected={expected:?}, current={}",
                graph.graph.version
            ),
            operation: "读取或修改画布".into(),
        });
    }
    Ok(())
}

fn graph_input(graph: CanvasGraphDocument) -> CanvasGraphInput {
    CanvasGraphInput {
        id: Some(graph.graph.id),
        name: graph.graph.name,
        graph_kind: graph.graph.graph_kind,
        viewport_json: graph.graph.viewport_json,
        nodes: graph
            .nodes
            .into_iter()
            .map(|n| CanvasNodeInput {
                client_id: n.id.to_string(),
                node_kind: n.node_kind,
                ref_type: n.ref_type,
                ref_id: n.ref_id,
                config_json: n.config_json,
                position_json: n.position_json,
                state: n.state,
            })
            .collect(),
        edges: graph
            .edges
            .into_iter()
            .map(|e| CanvasEdgeInput {
                source_client_id: e.source_node_id.to_string(),
                target_client_id: e.target_node_id.to_string(),
                source_port: e.source_port,
                target_port: e.target_port,
                role: e.role,
                config_json: e.config_json,
            })
            .collect(),
    }
}
