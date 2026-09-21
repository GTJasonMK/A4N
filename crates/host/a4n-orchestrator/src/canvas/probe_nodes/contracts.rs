use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_db_models::models::canvas::canvas_node;

use crate::canvas::CanvasNodeRunHandle;

pub struct CanvasProbeNodeExecutionInput<'a> {
    pub project_id: i64,
    pub graph_id: i64,
    pub node: &'a canvas_node::Model,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: Option<CanvasNodeRunHandle>,
    pub mode: &'a str,
}
