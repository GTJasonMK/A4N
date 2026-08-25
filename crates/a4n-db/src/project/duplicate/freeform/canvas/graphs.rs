use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::remap::remap_freeform_canvas_node_refs;
use crate::project::duplicate::freeform::{FreeformCanvasCopyMaps, FreeformCopyMaps};
use crate::project::duplicate::shared::remap_derived_required_id;
use a4n_core::models::canvas::{canvas_edge, canvas_graph, canvas_node};
use chrono::NaiveDateTime;
use sea_orm::sea_query::{Alias, CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

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
    let graphs = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(source_project_id))
        .order_by_asc(canvas_graph::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if graphs.is_empty() {
        return Ok(FreeformCanvasCopyMaps::default());
    }
    let graph_rows = graphs
        .iter()
        .map(|graph| canvas_graph::ActiveModel {
            project_id: Set(new_project_id),
            name: Set(graph.name.clone()),
            graph_kind: Set(graph.graph_kind.clone()),
            version: Set(graph.version),
            viewport_json: Set(graph.viewport_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let inserted_graphs = canvas_graph::Entity::insert_many(graph_rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted_graphs.len() != graphs.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的画布图数量与源记录不一致".to_string(),
            operation: "复制自由视频项目画布图".to_string(),
        });
    }
    let mut canvas_maps = FreeformCanvasCopyMaps::default();
    for (graph, copied) in graphs.iter().zip(&inserted_graphs) {
        canvas_maps.graph_ids.insert(graph.id, copied.id);
    }
    let graph_ids = graphs.iter().map(|graph| graph.id).collect::<Vec<_>>();
    let nodes = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.is_in(graph_ids.iter().copied()))
        .order_by_asc(canvas_node::Column::GraphId)
        .order_by_asc(canvas_node::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let node_rows = nodes
        .iter()
        .map(|node| {
            let graph_id = canvas_maps
                .graph_ids
                .get(&node.graph_id)
                .copied()
                .ok_or_else(|| ProjectDuplicateError::InvalidState {
                    current: format!("画布节点 {} 缺少图映射", node.id),
                    operation: "复制自由视频项目画布节点".to_string(),
                })?;
            Ok::<_, ProjectDuplicateError>(canvas_node::ActiveModel {
                graph_id: Set(graph_id),
                node_kind: Set(node.node_kind.clone()),
                ref_type: Set(node.ref_type.clone()),
                ref_id: Set(node.ref_id),
                config_json: Set(node.config_json.clone()),
                position_json: Set(node.position_json.clone()),
                state: Set(node.state.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !node_rows.is_empty() {
        let inserted_nodes = canvas_node::Entity::insert_many(node_rows)
            .exec_with_returning_many(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        if inserted_nodes.len() != nodes.len() {
            return Err(ProjectDuplicateError::InvalidState {
                current: "复制后的画布节点数量与源记录不一致".to_string(),
                operation: "复制自由视频项目画布节点".to_string(),
            });
        }
        for (node, copied) in nodes.iter().zip(&inserted_nodes) {
            canvas_maps.node_ids.insert(node.id, copied.id);
        }
    }

    let edges = canvas_edge::Entity::find()
        .filter(canvas_edge::Column::GraphId.is_in(graph_ids))
        .order_by_asc(canvas_edge::Column::GraphId)
        .order_by_asc(canvas_edge::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let edge_rows = edges
        .iter()
        .map(|edge| {
            let graph_id = remap_derived_required_id(
                &canvas_maps.graph_ids,
                edge.graph_id,
                "canvas_edge.graph_id",
            )?;
            Ok::<_, ProjectDuplicateError>(canvas_edge::ActiveModel {
                graph_id: Set(graph_id),
                source_node_id: Set(remap_derived_required_id(
                    &canvas_maps.node_ids,
                    edge.source_node_id,
                    "canvas_edge.source_node_id",
                )?),
                source_port: Set(edge.source_port.clone()),
                target_node_id: Set(remap_derived_required_id(
                    &canvas_maps.node_ids,
                    edge.target_node_id,
                    "canvas_edge.target_node_id",
                )?),
                target_port: Set(edge.target_port.clone()),
                role: Set(edge.role.clone()),
                config_json: Set(edge.config_json.clone()),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !edge_rows.is_empty() {
        canvas_edge::Entity::insert_many(edge_rows)
            .exec_without_returning(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
    }

    let mut full_maps = maps.clone();
    full_maps.graph_ids = canvas_maps.graph_ids.clone();
    full_maps.node_ids = canvas_maps.node_ids.clone();
    if nodes.is_empty() {
        return Ok(canvas_maps);
    }
    let mut ref_type_case = CaseStatement::new();
    let mut ref_id_case = CaseStatement::new();
    let mut config_json_case = CaseStatement::new();
    let mut copied_node_ids = Vec::with_capacity(nodes.len());
    for source_node in &nodes {
        let copied_node_id =
            remap_derived_required_id(&canvas_maps.node_ids, source_node.id, "canvas_node.id")?;
        let (ref_type, ref_id, config_json) = remap_freeform_canvas_node_refs(
            source_node.ref_type.clone(),
            source_node.ref_id,
            source_node.config_json.clone(),
            &full_maps,
            Some(&canvas_maps),
        )?;
        ref_type_case = ref_type_case.case(
            canvas_node::Column::Id.eq(copied_node_id),
            Expr::value(ref_type),
        );
        ref_id_case = ref_id_case.case(
            canvas_node::Column::Id.eq(copied_node_id),
            Expr::value(ref_id),
        );
        config_json_case = config_json_case.case(
            canvas_node::Column::Id.eq(copied_node_id),
            Expr::value(config_json).cast_as(Alias::new("json")),
        );
        copied_node_ids.push(copied_node_id);
    }
    let updated = canvas_node::Entity::update_many()
        .col_expr(
            canvas_node::Column::RefType,
            ref_type_case
                .finally(Expr::col(canvas_node::Column::RefType))
                .into(),
        )
        .col_expr(
            canvas_node::Column::RefId,
            ref_id_case
                .finally(Expr::col(canvas_node::Column::RefId))
                .into(),
        )
        .col_expr(
            canvas_node::Column::ConfigJson,
            config_json_case
                .finally(Expr::col(canvas_node::Column::ConfigJson))
                .into(),
        )
        .col_expr(canvas_node::Column::UpdatedAt, Expr::value(now))
        .filter(canvas_node::Column::Id.is_in(copied_node_ids))
        .exec(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if updated.rows_affected != nodes.len() as u64 {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的画布节点引用数量与源记录不一致".to_string(),
            operation: "回填自由视频项目画布节点引用".to_string(),
        });
    }
    Ok(canvas_maps)
}
