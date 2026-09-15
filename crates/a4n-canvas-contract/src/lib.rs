use a4n_domain::CanvasFrameKind;
use a4n_error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use a4n_api_contract::canvas::GenerateCanvasVideoInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasNodeRun {
    pub workflow_run_id: i64,
    pub finalize_on_success: bool,
}

pub type CanvasEditingNodeRun = CanvasNodeRun;

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
    pub fn from_node_kind(node_kind: &str) -> Result<Self, String> {
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
    pub fn summary(
        &self,
        status: &str,
        error_message: Option<String>,
    ) -> a4n_api_contract::canvas::CanvasRunSummary {
        a4n_api_contract::canvas::CanvasRunSummary {
            workflow_run_id: self.workflow_run_id,
            project_id: self.project_id,
            graph_id: self.graph_id,
            mode: self.mode.clone(),
            status: status.to_string(),
            error_message,
        }
    }
}

pub struct CanvasGraphNodeExecutionInput {
    pub kind: CanvasGraphNodeExecutionKind,
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub shared_run: CanvasNodeRun,
    pub mode: String,
}

#[async_trait]
pub trait CanvasGraphPreparationHostPort: Send + Sync {
    async fn verify_project_editor(&self, user_id: i64, project_id: i64) -> Result<(), String>;
}

#[async_trait]
pub trait CanvasGraphExecutionHostPort: Send + Sync {
    async fn execute_node(
        &self,
        input: CanvasGraphNodeExecutionInput,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, String>;
}

/// Runtime-facing Canvas graph preparation boundary.
#[async_trait]
pub trait CanvasGraphPreparationApplicationPort: Send + Sync {
    async fn prepare_graph(
        &self,
        input: CanvasGraphRunPreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String>;

    async fn prepare_single_node(
        &self,
        input: CanvasSingleNodeRunPreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String>;

    async fn prepare_resume(
        &self,
        input: CanvasGraphResumePreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String>;
}

/// Runtime-facing Canvas graph execution boundary.
#[async_trait]
pub trait CanvasGraphExecutionApplicationPort: Send + Sync {
    async fn run_prepared(
        &self,
        prepared: PreparedCanvasGraphRun,
        host_port: &dyn CanvasGraphExecutionHostPort,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, String>;
}

/// Runtime-facing Canvas graph failure-compensation boundary.
#[async_trait]
pub trait CanvasGraphFailureApplicationPort: Send + Sync {
    async fn fail_unsubmitted(&self, workflow_run_id: i64, error: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateCanvasImageInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    #[serde(default)]
    pub workflow_run_id: Option<i64>,
    pub prompt: String,
    pub config_id: Option<i64>,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub source_node_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct CanvasImageImportHostInput {
    pub project_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug)]
pub struct CanvasVideoImportHostInput {
    pub project_id: i64,
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub data_base64: Option<String>,
}

#[derive(Debug)]
pub struct CanvasAudioImportHostInput {
    pub project_id: i64,
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub data_base64: Option<String>,
}

#[async_trait]
pub trait CanvasImageImportHostPort: Send + Sync {
    async fn import_canvas_image(
        &self,
        input: CanvasImageImportHostInput,
    ) -> Result<a4n_api_contract::image::GeneratedImageSummary, AppError>;
}

#[async_trait]
pub trait CanvasVideoImportHostPort: Send + Sync {
    async fn import_canvas_video(&self, input: CanvasVideoImportHostInput)
    -> Result<i64, AppError>;

    async fn delete_canvas_video(&self, video_id: i64) -> Result<(), AppError>;
}

#[derive(Debug)]
pub struct CanvasImageImportApplicationResult {
    pub image: a4n_api_contract::image::GeneratedImageSummary,
    pub asset: a4n_api_contract::canvas::CanvasMediaAssetSummary,
}

#[derive(Debug)]
pub struct CanvasVideoImportApplicationResult {
    pub video: a4n_api_contract::video::GeneratedVideoSummary,
    pub asset: a4n_api_contract::canvas::CanvasMediaAssetSummary,
}

/// Host-facing Canvas image import boundary.
#[async_trait]
pub trait CanvasImageImportApplicationPort: Send + Sync {
    async fn import_image(
        &self,
        host_port: &dyn CanvasImageImportHostPort,
        user_id: i64,
        input: CanvasImageImportHostInput,
    ) -> Result<CanvasImageImportApplicationResult, AppError>;
}

/// Host-facing Canvas video import boundary.
#[async_trait]
pub trait CanvasVideoImportApplicationPort: Send + Sync {
    async fn import_video(
        &self,
        host_port: &dyn CanvasVideoImportHostPort,
        user_id: i64,
        input: CanvasVideoImportHostInput,
    ) -> Result<CanvasVideoImportApplicationResult, AppError>;
}

/// Host-facing Canvas audio import boundary.
#[async_trait]
pub trait CanvasAudioImportApplicationPort: Send + Sync {
    async fn import_audio(
        &self,
        user_id: i64,
        input: CanvasAudioImportHostInput,
    ) -> Result<a4n_api_contract::canvas::CanvasMediaAssetSummary, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasEditingNodeApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub shared_run: Option<CanvasEditingNodeRun>,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasProbeNodeApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub shared_run: Option<CanvasNodeRun>,
    pub mode: String,
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

#[async_trait]
pub trait CanvasEditingNodeApplicationPort: Send + Sync {
    async fn run_editing(
        &self,
        input: CanvasEditingNodeApplicationInput,
        host_port: &dyn CanvasEditingNodeHostPort,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasImageNodeApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub shared_run: Option<CanvasNodeRun>,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasVideoNodeApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub shared_run: Option<CanvasNodeRun>,
    pub mode: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasPreviewNodeApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasRenderTextNodeApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub shared_run: Option<CanvasNodeRun>,
    pub mode: String,
}

#[async_trait]
pub trait CanvasRenderTextNodeApplicationPort: Send + Sync {
    async fn run_render_text(
        &self,
        input: CanvasRenderTextNodeApplicationInput,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasLlmTextNodeApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub shared_run: Option<CanvasNodeRun>,
    pub mode: String,
}

#[async_trait]
pub trait CanvasProbeNodeApplicationPort: Send + Sync {
    async fn run_probe(
        &self,
        input: CanvasProbeNodeApplicationInput,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasOutputNodeApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub shared_run: Option<CanvasNodeRun>,
    pub mode: String,
}

#[async_trait]
pub trait CanvasOutputNodeApplicationPort: Send + Sync {
    async fn run_output(
        &self,
        input: CanvasOutputNodeApplicationInput,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError>;
}
