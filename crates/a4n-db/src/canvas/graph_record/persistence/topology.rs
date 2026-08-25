use std::collections::BTreeMap;

use a4n_core::models::canvas::{canvas_edge, canvas_node};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

use super::super::{types::CanvasGraphRecordInput, validation::required_canvas_record_text};
use crate::canvas::CanvasDbError;

const NODE_STATE_READY: &str = "ready";

pub(super) async fn replace_canvas_graph_topology<C>(
    db: &C,
    graph_id: i64,
    input: &CanvasGraphRecordInput,
    now: chrono::NaiveDateTime,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    canvas_edge::Entity::delete_many()
        .filter(canvas_edge::Column::GraphId.eq(graph_id))
        .exec(db)
        .await?;
    canvas_node::Entity::delete_many()
        .filter(canvas_node::Column::GraphId.eq(graph_id))
        .exec(db)
        .await?;

    let node_rows = input
        .nodes
        .iter()
        .map(|node| {
            Ok(canvas_node::ActiveModel {
                graph_id: Set(graph_id),
                node_kind: Set(required_canvas_record_text(
                    &node.node_kind,
                    "node.node_kind",
                )?),
                ref_type: Set(node.ref_type.clone()),
                ref_id: Set(node.ref_id),
                config_json: Set(node.config_json.clone().unwrap_or_else(|| json!({}))),
                position_json: Set(node.position_json.clone()),
                state: Set(node
                    .state
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(NODE_STATE_READY)
                    .to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, CanvasDbError>>()?;
    let mut node_ids = BTreeMap::<String, i64>::new();
    if !node_rows.is_empty() {
        let inserted_nodes = canvas_node::Entity::insert_many(node_rows)
            .exec_with_returning_many(db)
            .await?;
        if inserted_nodes.len() != input.nodes.len() {
            return Err(CanvasDbError::InvalidArgument(format!(
                "画布节点批量写入返回行数不一致: inserted={}, expected={}",
                inserted_nodes.len(),
                input.nodes.len()
            )));
        }
        for (node, row) in input.nodes.iter().zip(inserted_nodes) {
            node_ids.insert(
                required_canvas_record_text(&node.client_id, "node.client_id")?,
                row.id,
            );
        }
    }

    let edge_rows = input
        .edges
        .iter()
        .map(|edge| {
            let source_node_id =
                resolve_canvas_record_edge_node_id(&edge.source_client_id, &node_ids, "source")?;
            let target_node_id =
                resolve_canvas_record_edge_node_id(&edge.target_client_id, &node_ids, "target")?;
            Ok(canvas_edge::ActiveModel {
                graph_id: Set(graph_id),
                source_node_id: Set(source_node_id),
                source_port: Set(required_canvas_record_text(
                    &edge.source_port,
                    "edge.source_port",
                )?),
                target_node_id: Set(target_node_id),
                target_port: Set(required_canvas_record_text(
                    &edge.target_port,
                    "edge.target_port",
                )?),
                role: Set(required_canvas_record_text(&edge.role, "edge.role")?),
                config_json: Set(edge.config_json.clone().unwrap_or_else(|| json!({}))),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, CanvasDbError>>()?;
    if !edge_rows.is_empty() {
        canvas_edge::Entity::insert_many(edge_rows)
            .exec_without_returning(db)
            .await?;
    }
    Ok(())
}

fn resolve_canvas_record_edge_node_id(
    client_id: &str,
    node_ids: &BTreeMap<String, i64>,
    label: &str,
) -> Result<i64, CanvasDbError> {
    node_ids
        .get(client_id.trim())
        .copied()
        .ok_or_else(|| CanvasDbError::InvalidArgument(format!("{label} 节点不存在: {client_id}")))
}
