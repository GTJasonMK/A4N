use a4n_core::{error::AppError, models::canvas::canvas_node};
use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_domain::CanvasFrameKind;
use async_trait::async_trait;

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
    pub node: &'a canvas_node::Model,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: Option<CanvasNodeRunHandle>,
    pub mode: &'a str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasExtractFrameHostInput {
    pub user_id: i64,
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub video_id: i64,
    pub frame_kind: CanvasFrameKind,
    pub at_secs: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasTrimVideoHostInput {
    pub user_id: i64,
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub video_id: i64,
    pub start_secs: f32,
    pub end_secs: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasAssembleVideosHostInput {
    pub user_id: i64,
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub video_ids: Vec<i64>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasGeneratedImageArtifact {
    pub generated_image_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasGeneratedVideoArtifact {
    pub generated_video_id: i64,
}

#[async_trait]
pub trait CanvasEditingNodeHostPort: Send + Sync {
    async fn extract_frame(
        &self,
        input: CanvasExtractFrameHostInput,
    ) -> Result<CanvasGeneratedImageArtifact, AppError>;

    async fn trim_video(
        &self,
        input: CanvasTrimVideoHostInput,
    ) -> Result<CanvasGeneratedVideoArtifact, AppError>;

    async fn assemble_videos(
        &self,
        input: CanvasAssembleVideosHostInput,
    ) -> Result<CanvasGeneratedVideoArtifact, AppError>;
}
