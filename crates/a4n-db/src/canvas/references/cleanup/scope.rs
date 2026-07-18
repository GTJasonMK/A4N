use crate::canvas::CanvasDbError;
use a4n_core::models::canvas::{canvas_graph, canvas_node};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(super) async fn canvas_nodes_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<canvas_node::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    let graph_ids = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(project_id))
        .all(db)
        .await?
        .into_iter()
        .map(|graph| graph.id)
        .collect::<Vec<_>>();
    canvas_nodes_for_graph_ids(db, graph_ids).await
}

pub(super) async fn canvas_nodes_for_projects<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<canvas_node::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    let graph_ids = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.is_in(project_ids.to_vec()))
        .all(db)
        .await?
        .into_iter()
        .map(|graph| graph.id)
        .collect::<Vec<_>>();
    canvas_nodes_for_graph_ids(db, graph_ids).await
}

async fn canvas_nodes_for_graph_ids<C>(
    db: &C,
    graph_ids: Vec<i64>,
) -> Result<Vec<canvas_node::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    if graph_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.is_in(graph_ids))
        .all(db)
        .await?)
}
