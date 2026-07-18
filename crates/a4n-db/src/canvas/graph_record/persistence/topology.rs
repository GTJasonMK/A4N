use std::collections::BTreeMap;

use a4n_core::models::canvas::{canvas_edge, canvas_node};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
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

    let mut node_ids = BTreeMap::<String, i64>::new();
    for node in &input.nodes {
        let client_id = required_canvas_record_text(&node.client_id, "node.client_id")?;
        let ref_type = node
            .ref_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let row = canvas_node::ActiveModel {
            graph_id: Set(graph_id),
            node_kind: Set(required_canvas_record_text(
                &node.node_kind,
                "node.node_kind",
            )?),
            ref_type: Set(ref_type),
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
        }
        .insert(db)
        .await?;
        node_ids.insert(client_id, row.id);
    }

    for edge in &input.edges {
        let source_node_id = resolve_canvas_record_edge_node_id(
            edge.source_node_id,
            &edge.source_client_id,
            &node_ids,
            "source",
        )?;
        let target_node_id = resolve_canvas_record_edge_node_id(
            edge.target_node_id,
            &edge.target_client_id,
            &node_ids,
            "target",
        )?;
        canvas_edge::ActiveModel {
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
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

fn resolve_canvas_record_edge_node_id(
    explicit_id: Option<i64>,
    client_id: &str,
    node_ids: &BTreeMap<String, i64>,
    label: &str,
) -> Result<i64, CanvasDbError> {
    let _ = explicit_id;
    node_ids
        .get(client_id.trim())
        .copied()
        .ok_or_else(|| CanvasDbError::InvalidArgument(format!("{label} 节点不存在: {client_id}")))
}
