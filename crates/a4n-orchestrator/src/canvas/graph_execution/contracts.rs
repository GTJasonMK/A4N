use a4n_core::models::canvas::canvas_node;
use a4n_db::canvas::CanvasGraphRecordDocument;
use async_trait::async_trait;

use crate::canvas::{CanvasNodeRunHandle, CanvasNodeRunSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasGraphNodeExecutionKind {
    RenderText,
    LlmText,
    GenerateImage,
    GenerateVideo,
    ExtractFrame,
    TrimVideo,
    AssembleVideo,
    Probe,
    TextOutput,
    ImageOutput,
    VideoOutput,
    AudioOutput,
}

impl CanvasGraphNodeExecutionKind {
    pub(crate) fn from_node_kind(node_kind: &str) -> Result<Self, String> {
        match node_kind {
            "RenderTextNode" => Ok(Self::RenderText),
            "LlmTextNode" => Ok(Self::LlmText),
            "GenerateImageNode" => Ok(Self::GenerateImage),
            "GenerateVideoNode" => Ok(Self::GenerateVideo),
            "ExtractFrameNode" => Ok(Self::ExtractFrame),
            "TrimVideoNode" => Ok(Self::TrimVideo),
            "AssembleVideoNode" => Ok(Self::AssembleVideo),
            "TextProbeNode" | "ImageProbeNode" | "VideoProbeNode" | "AudioProbeNode" => {
                Ok(Self::Probe)
            }
            "TextOutputNode" => Ok(Self::TextOutput),
            "ImageOutputNode" => Ok(Self::ImageOutput),
            "VideoOutputNode" => Ok(Self::VideoOutput),
            "AudioOutputNode" => Ok(Self::AudioOutput),
            other => Err(format!("画布节点类型不支持执行: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasGraphRunPreparationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub target_node_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasSingleNodeRunPreparationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasGraphResumePreparationInput {
    pub user_id: i64,
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedCanvasGraphRun {
    pub workflow_run_id: i64,
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub mode: String,
    pub runnable_node_ids: Vec<i64>,
}

impl PreparedCanvasGraphRun {
    pub fn summary(&self, status: &str, error_message: Option<String>) -> CanvasNodeRunSummary {
        CanvasNodeRunSummary {
            workflow_run_id: self.workflow_run_id,
            project_id: self.project_id,
            graph_id: self.graph_id,
            mode: self.mode.clone(),
            status: status.to_string(),
            error_message,
        }
    }
}

pub struct CanvasGraphNodeExecutionInput<'a> {
    pub kind: CanvasGraphNodeExecutionKind,
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node: &'a canvas_node::Model,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: CanvasNodeRunHandle,
    pub mode: &'a str,
}

#[async_trait]
pub trait CanvasGraphExecutionHostPort: Send + Sync {
    async fn verify_project_editor(&self, user_id: i64, project_id: i64) -> Result<(), String>;

    async fn execute_node(
        &self,
        input: CanvasGraphNodeExecutionInput<'_>,
    ) -> Result<CanvasNodeRunSummary, String>;
}
