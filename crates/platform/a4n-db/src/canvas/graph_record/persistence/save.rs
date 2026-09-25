use sea_orm::{ConnectionTrait, TransactionTrait};

use super::super::{
    references::validate_canvas_graph_record_references,
    types::{CanvasGraphRecordInput, CanvasGraphRecordSaveResult},
    validation::validate_canvas_graph_record_input,
};
use super::{
    context::require_canvas_project_record, document::canvas_graph_record_document,
    graph_write::upsert_canvas_graph_record, topology::replace_canvas_graph_topology,
};
use crate::canvas::CanvasDbError;

pub async fn save_canvas_graph_record<C>(
    db: &C,
    project_id: i64,
    input: CanvasGraphRecordInput,
    now: chrono::NaiveDateTime,
) -> Result<CanvasGraphRecordSaveResult, CanvasDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let project = require_canvas_project_record(db, project_id).await?;
    validate_canvas_graph_record_input(&input)?;
    let tx = db.begin().await?;
    validate_canvas_graph_record_references(&tx, project_id, &input).await?;
    let graph = upsert_canvas_graph_record(&tx, &project, project_id, &input, now).await?;
    let node_ids = replace_canvas_graph_topology(&tx, graph.id, &input, now).await?;
    let document = canvas_graph_record_document(&tx, graph).await?;
    tx.commit().await?;
    Ok(CanvasGraphRecordSaveResult { document, node_ids })
}
