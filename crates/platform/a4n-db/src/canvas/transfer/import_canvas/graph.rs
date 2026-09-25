use super::super::remap::remap_video_studio_required_id;
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps, VideoStudioProjectImportRows,
};
use a4n_db_models::models::canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset};
use sea_orm::{ConnectionTrait, EntityTrait, Set};

pub(crate) async fn import_video_studio_canvas_text_assets<C>(
    db: &C,
    project_id: i64,
    assets: &[canvas_text_asset::Model],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    if assets.is_empty() {
        return Ok(());
    }
    let rows = assets
        .iter()
        .map(|asset| canvas_text_asset::ActiveModel {
            project_id: Set(project_id),
            title: Set(asset.title.clone()),
            text_kind: Set(asset.text_kind.clone()),
            content: Set(asset.content.clone()),
            content_hash: Set(asset.content_hash.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let inserted = canvas_text_asset::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if inserted.len() != assets.len() {
        return Err(VideoStudioProjectImportError::InvalidPackage(
            "画布文本资产批量写入返回行数不一致".to_string(),
        ));
    }
    for (asset, row) in assets.iter().zip(inserted) {
        maps.text_asset_ids.insert(asset.id, row.id);
    }
    Ok(())
}

pub(crate) async fn import_video_studio_canvas_graphs<C>(
    db: &C,
    project_id: i64,
    rows: &VideoStudioProjectImportRows,
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    if !rows.canvas_graphs.is_empty() {
        let graph_rows = rows
            .canvas_graphs
            .iter()
            .map(|graph| canvas_graph::ActiveModel {
                project_id: Set(project_id),
                name: Set(graph.name.clone()),
                graph_kind: Set(graph.graph_kind.clone()),
                version: Set(graph.version),
                viewport_json: Set(graph.viewport_json.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let inserted = canvas_graph::Entity::insert_many(graph_rows)
            .exec_with_returning_many(db)
            .await
            .map_err(VideoStudioProjectImportError::Database)?;
        if inserted.len() != rows.canvas_graphs.len() {
            return Err(VideoStudioProjectImportError::InvalidPackage(
                "画布图批量写入返回行数不一致".to_string(),
            ));
        }
        for (graph, row) in rows.canvas_graphs.iter().zip(inserted) {
            maps.graph_ids.insert(graph.id, row.id);
        }
    }

    let node_rows = rows
        .canvas_nodes
        .iter()
        .map(|node| {
            let graph_id = remap_video_studio_required_id(
                &maps.graph_ids,
                node.graph_id,
                "canvas_node.graph_id",
            )?;
            Ok::<_, VideoStudioProjectImportError>(canvas_node::ActiveModel {
                graph_id: Set(graph_id),
                node_kind: Set(node.node_kind.clone()),
                ref_type: Set(None),
                ref_id: Set(None),
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
        let inserted = canvas_node::Entity::insert_many(node_rows)
            .exec_with_returning_many(db)
            .await
            .map_err(VideoStudioProjectImportError::Database)?;
        if inserted.len() != rows.canvas_nodes.len() {
            return Err(VideoStudioProjectImportError::InvalidPackage(
                "画布节点批量写入返回行数不一致".to_string(),
            ));
        }
        for (node, row) in rows.canvas_nodes.iter().zip(inserted) {
            maps.node_ids.insert(node.id, row.id);
        }
    }

    let edge_rows = rows
        .canvas_edges
        .iter()
        .map(|edge| {
            Ok::<_, VideoStudioProjectImportError>(canvas_edge::ActiveModel {
                graph_id: Set(remap_video_studio_required_id(
                    &maps.graph_ids,
                    edge.graph_id,
                    "canvas_edge.graph_id",
                )?),
                source_node_id: Set(remap_video_studio_required_id(
                    &maps.node_ids,
                    edge.source_node_id,
                    "canvas_edge.source_node_id",
                )?),
                source_port: Set(edge.source_port.clone()),
                target_node_id: Set(remap_video_studio_required_id(
                    &maps.node_ids,
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
            .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}
