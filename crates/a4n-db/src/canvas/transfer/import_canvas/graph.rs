use super::super::remap::remap_video_studio_required_id;
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps, VideoStudioProjectImportRows,
};
use a4n_core::models::canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(in crate::canvas::transfer) async fn import_video_studio_canvas_text_assets<C>(
    db: &C,
    project_id: i64,
    assets: &[canvas_text_asset::Model],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for asset in assets {
        let row = canvas_text_asset::ActiveModel {
            project_id: Set(project_id),
            title: Set(asset.title.clone()),
            text_kind: Set(asset.text_kind.clone()),
            content: Set(asset.content.clone()),
            content_hash: Set(asset.content_hash.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        maps.text_asset_ids.insert(asset.id, row.id);
    }
    Ok(())
}

pub(in crate::canvas::transfer) async fn import_video_studio_canvas_graphs<C>(
    db: &C,
    project_id: i64,
    rows: &VideoStudioProjectImportRows,
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for graph in &rows.canvas_graphs {
        let row = canvas_graph::ActiveModel {
            project_id: Set(project_id),
            name: Set(graph.name.clone()),
            graph_kind: Set(graph.graph_kind.clone()),
            version: Set(graph.version),
            viewport_json: Set(graph.viewport_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        maps.graph_ids.insert(graph.id, row.id);
    }

    for node in &rows.canvas_nodes {
        let graph_id =
            remap_video_studio_required_id(&maps.graph_ids, node.graph_id, "canvas_node.graph_id")?;
        let row = canvas_node::ActiveModel {
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
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        maps.node_ids.insert(node.id, row.id);
    }

    for edge in &rows.canvas_edges {
        canvas_edge::ActiveModel {
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
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}
