use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::remap::remap_freeform_canvas_node_refs;
use crate::project::duplicate::freeform::{FreeformCanvasCopyMaps, FreeformCopyMaps};
use crate::project::duplicate::shared::remap_derived_required_id;
use a4n_core::models::canvas::{canvas_edge, canvas_graph, canvas_node};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::HashMap;

pub(in crate::project::duplicate::freeform) async fn copy_freeform_canvas_graphs_record<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<FreeformCanvasCopyMaps, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let mut canvas_maps = FreeformCanvasCopyMaps::default();
    let mut copied_nodes = Vec::new();
    let graphs = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(source_project_id))
        .order_by_asc(canvas_graph::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    for graph in graphs {
        let new_graph = canvas_graph::ActiveModel {
            project_id: Set(new_project_id),
            name: Set(graph.name),
            graph_kind: Set(graph.graph_kind),
            version: Set(graph.version),
            viewport_json: Set(graph.viewport_json),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        canvas_maps.graph_ids.insert(graph.id, new_graph.id);
        let nodes = canvas_node::Entity::find()
            .filter(canvas_node::Column::GraphId.eq(graph.id))
            .order_by_asc(canvas_node::Column::Id)
            .all(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        let mut node_id_map = HashMap::new();
        for node in nodes {
            let row = canvas_node::ActiveModel {
                graph_id: Set(new_graph.id),
                node_kind: Set(node.node_kind.clone()),
                ref_type: Set(node.ref_type.clone()),
                ref_id: Set(node.ref_id),
                config_json: Set(node.config_json.clone()),
                position_json: Set(node.position_json.clone()),
                state: Set(node.state.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
            node_id_map.insert(node.id, row.id);
            canvas_maps.node_ids.insert(node.id, row.id);
            copied_nodes.push((node, row.id));
        }
        let edges = canvas_edge::Entity::find()
            .filter(canvas_edge::Column::GraphId.eq(graph.id))
            .order_by_asc(canvas_edge::Column::Id)
            .all(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        for edge in edges {
            canvas_edge::ActiveModel {
                graph_id: Set(new_graph.id),
                source_node_id: Set(remap_derived_required_id(
                    &node_id_map,
                    edge.source_node_id,
                    "canvas_edge.source_node_id",
                )?),
                source_port: Set(edge.source_port),
                target_node_id: Set(remap_derived_required_id(
                    &node_id_map,
                    edge.target_node_id,
                    "canvas_edge.target_node_id",
                )?),
                target_port: Set(edge.target_port),
                role: Set(edge.role),
                config_json: Set(edge.config_json),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        }
    }

    let mut full_maps = maps.clone();
    full_maps.graph_ids = canvas_maps.graph_ids.clone();
    full_maps.node_ids = canvas_maps.node_ids.clone();
    for (source_node, copied_node_id) in copied_nodes {
        let (ref_type, ref_id, config_json) = remap_freeform_canvas_node_refs(
            source_node.ref_type,
            source_node.ref_id,
            source_node.config_json,
            &full_maps,
            Some(&canvas_maps),
        )?;
        canvas_node::ActiveModel {
            id: Set(copied_node_id),
            ref_type: Set(ref_type),
            ref_id: Set(ref_id),
            config_json: Set(config_json),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(canvas_maps)
}
