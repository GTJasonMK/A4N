use crate::canvas::CanvasDbError;
use a4n_core::models::canvas::{canvas_graph, canvas_node};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
};

#[derive(Debug, FromQueryResult)]
pub(super) struct CanvasNodeReferenceRow {
    pub(super) id: i64,
    pub(super) ref_type: Option<String>,
    pub(super) ref_id: Option<i64>,
    pub(super) config_json: serde_json::Value,
}

pub(super) async fn canvas_nodes_for_projects<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<CanvasNodeReferenceRow>, CanvasDbError>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    let graph_ids = canvas_graph::Entity::find()
        .select_only()
        .column(canvas_graph::Column::Id)
        .filter(canvas_graph::Column::ProjectId.is_in(project_ids.to_vec()))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    canvas_nodes_for_graph_ids(db, graph_ids).await
}

async fn canvas_nodes_for_graph_ids<C>(
    db: &C,
    graph_ids: Vec<i64>,
) -> Result<Vec<CanvasNodeReferenceRow>, CanvasDbError>
where
    C: ConnectionTrait,
{
    if graph_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(canvas_node::Entity::find()
        .select_only()
        .column(canvas_node::Column::Id)
        .column(canvas_node::Column::RefType)
        .column(canvas_node::Column::RefId)
        .column(canvas_node::Column::ConfigJson)
        .filter(canvas_node::Column::GraphId.is_in(graph_ids))
        .into_model::<CanvasNodeReferenceRow>()
        .all(db)
        .await?)
}
