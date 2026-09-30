use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_domain::CanvasFrameKind;
use a4n_domain::canvas::CanvasNodeRecord;

use crate::canvas::CanvasNodeRunHandle;

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasExtractFrameNodeInput {
    pub video_id: i64,
    pub frame_kind: CanvasFrameKind,
    pub at_secs: Option<f32>,
    pub canvas_text_asset_id: Option<i64>,
    pub source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasTrimVideoNodeInput {
    pub video_id: i64,
    pub start_secs: f32,
    pub end_secs: f32,
    pub canvas_text_asset_id: Option<i64>,
    pub source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasAssembleVideoNodeInput {
    pub video_ids: Vec<i64>,
    pub title: Option<String>,
    pub canvas_text_asset_id: Option<i64>,
    pub source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasEditingNodeInput {
    ExtractFrame(CanvasExtractFrameNodeInput),
    TrimVideo(CanvasTrimVideoNodeInput),
    AssembleVideo(CanvasAssembleVideoNodeInput),
}

pub struct CanvasEditingNodeExecutionInput<'a> {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node: &'a CanvasNodeRecord,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: Option<CanvasNodeRunHandle>,
    pub mode: &'a str,
}
