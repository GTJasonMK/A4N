use a4n_db_models::models::canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use super::super::records::{
    canvas_edge_record, canvas_graph_record, canvas_node_record, canvas_text_asset_record,
};
use super::context::{
    CanvasProjectIdentity, default_canvas_record_graph_kind, require_canvas_project_record,
};
use super::media_inputs::populate_media_input_metadata;
use crate::canvas::{CanvasDbError, CanvasGraphRecordDocument};

pub async fn get_or_create_canvas_graph_record<C>(
    db: &C,
    project_id: i64,
    graph_id: Option<i64>,
    now: chrono::NaiveDateTime,
) -> Result<CanvasGraphRecordDocument, CanvasDbError>
where
    C: ConnectionTrait,
{
    let project = require_canvas_project_record(db, project_id).await?;
    let graph = match graph_id {
        Some(graph_id) => load_canvas_graph_record(db, project_id, graph_id).await?,
        None => {
            let graph_kind = default_canvas_record_graph_kind(&project)?;
            match canvas_graph::Entity::find()
                .filter(canvas_graph::Column::ProjectId.eq(project_id))
                .filter(canvas_graph::Column::GraphKind.eq(graph_kind))
                .order_by_asc(canvas_graph::Column::Id)
                .one(db)
                .await?
            {
                Some(graph) => graph,
                None => insert_default_canvas_graph_record(db, &project, now).await?,
            }
        }
    };
    canvas_graph_record_document(db, graph).await
}

async fn insert_default_canvas_graph_record<C>(
    db: &C,
    project: &CanvasProjectIdentity,
    now: chrono::NaiveDateTime,
) -> Result<canvas_graph::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    canvas_graph::ActiveModel {
        project_id: Set(project.id),
        name: Set("视频画布".to_string()),
        graph_kind: Set(default_canvas_record_graph_kind(project)?.to_string()),
        version: Set(1),
        viewport_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(CanvasDbError::Database)
}

async fn load_canvas_graph_record<C>(
    db: &C,
    project_id: i64,
    graph_id: i64,
) -> Result<canvas_graph::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let graph = canvas_graph::Entity::find_by_id(graph_id)
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("画布不存在: {graph_id}")))?;
    if graph.project_id != project_id {
        return Err(CanvasDbError::Forbidden("画布不属于当前项目".to_string()));
    }
    Ok(graph)
}

pub(super) async fn canvas_graph_record_document<C>(
    db: &C,
    graph: canvas_graph::Model,
) -> Result<CanvasGraphRecordDocument, CanvasDbError>
where
    C: ConnectionTrait,
{
    let (nodes_result, edges_result, text_assets_result) = futures::join!(
        canvas_node::Entity::find()
            .filter(canvas_node::Column::GraphId.eq(graph.id))
            .order_by_asc(canvas_node::Column::Id)
            .all(db),
        canvas_edge::Entity::find()
            .filter(canvas_edge::Column::GraphId.eq(graph.id))
            .order_by_asc(canvas_edge::Column::Id)
            .all(db),
        canvas_text_asset::Entity::find()
            .filter(canvas_text_asset::Column::ProjectId.eq(graph.project_id))
            .order_by_desc(canvas_text_asset::Column::UpdatedAt)
            .all(db),
    );
    let mut nodes = nodes_result?;
    let edges = edges_result?;
    let text_assets = text_assets_result?;
    populate_media_input_metadata(db, graph.project_id, &mut nodes).await?;
    Ok(CanvasGraphRecordDocument {
        graph: canvas_graph_record(graph),
        nodes: nodes.into_iter().map(canvas_node_record).collect(),
        edges: edges.into_iter().map(canvas_edge_record).collect(),
        text_assets: text_assets
            .into_iter()
            .map(canvas_text_asset_record)
            .collect(),
    })
}
