use std::collections::BTreeMap;

use a4n_db_models::models::canvas::{canvas_edge, canvas_node};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

use super::super::{types::CanvasGraphRecordInput, validation::required_canvas_record_text};
use super::media_inputs::clear_media_input_metadata;
use crate::canvas::CanvasDbError;

const NODE_STATE_READY: &str = "ready";

/// Preserve saved node IDs (client_id) and unchanged edge IDs during edits.
pub(super) async fn replace_canvas_graph_topology<C>(
    db: &C,
    graph_id: i64,
    input: &CanvasGraphRecordInput,
    now: chrono::NaiveDateTime,
) -> Result<BTreeMap<String, i64>, CanvasDbError>
where
    C: ConnectionTrait,
{
    let old_nodes = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.eq(graph_id))
        .all(db)
        .await?
        .into_iter()
        .map(|node| (node.id.to_string(), node))
        .collect::<BTreeMap<_, _>>();
    let old_edges = canvas_edge::Entity::find()
        .filter(canvas_edge::Column::GraphId.eq(graph_id))
        .all(db)
        .await?
        .into_iter()
        .map(|edge| {
            (
                (
                    edge.source_node_id,
                    edge.target_node_id,
                    edge.source_port.clone(),
                    edge.target_port.clone(),
                    edge.role.clone(),
                ),
                edge,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut node_ids = BTreeMap::<String, i64>::new();
    for node in &input.nodes {
        let client_id = required_canvas_record_text(&node.client_id, "node.client_id")?;
        let mut config = node.config_json.clone().unwrap_or_else(|| json!({}));
        let kind = required_canvas_record_text(&node.node_kind, "node.node_kind")?;
        clear_media_input_metadata(&kind, &mut config)?;
        let state = node
            .state
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(NODE_STATE_READY)
            .to_string();
        let old = old_nodes.get(&client_id);
        if let Some(old) = old
            && old.node_kind == kind
            && old.ref_type == node.ref_type
            && old.ref_id == node.ref_id
            && old.config_json == config
            && old.position_json == node.position_json
            && old.state == state
        {
            node_ids.insert(client_id, old.id);
            continue;
        }
        let mut active: canvas_node::ActiveModel = old.cloned().map(Into::into).unwrap_or_default();
        active.graph_id = Set(graph_id);
        active.node_kind = Set(kind);
        active.ref_type = Set(node.ref_type.clone());
        active.ref_id = Set(node.ref_id);
        active.config_json = Set(config);
        active.position_json = Set(node.position_json.clone());
        active.state = Set(state);
        active.updated_at = Set(now);
        let saved = if old.is_some() {
            active.update(db).await?
        } else {
            active.created_at = Set(now);
            active.insert(db).await?
        };
        node_ids.insert(client_id, saved.id);
    }
    let mut retained_edges = Vec::new();
    for edge in &input.edges {
        let source =
            resolve_canvas_record_edge_node_id(&edge.source_client_id, &node_ids, "source")?;
        let target =
            resolve_canvas_record_edge_node_id(&edge.target_client_id, &node_ids, "target")?;
        let source_port = required_canvas_record_text(&edge.source_port, "edge.source_port")?;
        let target_port = required_canvas_record_text(&edge.target_port, "edge.target_port")?;
        let role = required_canvas_record_text(&edge.role, "edge.role")?;
        let config = edge.config_json.clone().unwrap_or_else(|| json!({}));
        let old = old_edges.get(&(
            source,
            target,
            source_port.clone(),
            target_port.clone(),
            role.clone(),
        ));
        let id = if let Some(old) = old {
            if old.config_json != config {
                let mut active: canvas_edge::ActiveModel = old.clone().into();
                active.config_json = Set(config);
                active.update(db).await?;
            }
            old.id
        } else {
            canvas_edge::ActiveModel {
                graph_id: Set(graph_id),
                source_node_id: Set(source),
                target_node_id: Set(target),
                source_port: Set(source_port),
                target_port: Set(target_port),
                role: Set(role),
                config_json: Set(config),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id
        };
        retained_edges.push(id);
    }
    canvas_edge::Entity::delete_many()
        .filter(canvas_edge::Column::GraphId.eq(graph_id))
        .filter(canvas_edge::Column::Id.is_not_in(retained_edges))
        .exec(db)
        .await?;
    canvas_node::Entity::delete_many()
        .filter(canvas_node::Column::GraphId.eq(graph_id))
        .filter(canvas_node::Column::Id.is_not_in(node_ids.values().copied()))
        .exec(db)
        .await?;
    Ok(node_ids)
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
