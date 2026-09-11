use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_db_models::models::canvas::canvas_node;

use crate::canvas::CanvasNodeRunHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasImageNodeInput {
    pub prompt: String,
    pub config_id: Option<i64>,
    pub width: i32,
    pub height: i32,
    pub canvas_text_asset_id: Option<i64>,
    pub source_node_ids: Vec<i64>,
}

pub struct CanvasImageNodeExecutionInput<'a> {
    pub requested_by_user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node: &'a canvas_node::Model,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: Option<CanvasNodeRunHandle>,
    pub mode: &'a str,
}
