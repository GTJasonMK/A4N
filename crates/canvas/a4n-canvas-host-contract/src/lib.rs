//! Runtime-facing contracts for Canvas node execution and request previews.

use a4n_api_contract::canvas::CanvasRunSummary;
use a4n_canvas_contract::{
    CanvasImageNodeApplicationInput, CanvasLlmTextNodeApplicationInput,
    CanvasPreviewNodeApplicationInput, CanvasVideoNodeApplicationInput,
};
use a4n_error::AppError;
use a4n_image_host_contract::{ImageGenerationHostPort, ImageGenerationProgress};
use a4n_llm_provider_contract::provider::{ChatRequest, ChatResponse};
use a4n_provider_contract::VideoProviderConfig;
use a4n_video_host_contract::{
    CanvasVideoImageInput, CanvasVideoPreviewHostPort, CanvasVideoResolvedImages,
};
use a4n_video_provider_contract::{VideoGenerationResult, VideoJobState};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum CanvasNodeRequestPreview {
    Image(a4n_canvas_contract::GenerateCanvasImageInput),
    Video(a4n_canvas_contract::GenerateCanvasVideoInput),
    Llm(ChatRequest),
}

#[derive(Debug)]
pub struct CanvasTextGenerationLlmCall {
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub config_id: Option<i64>,
    pub request: ChatRequest,
}

#[async_trait]
pub trait CanvasTextGenerationLlmPort: Send + Sync {
    async fn max_output_tokens(&self, config_id: Option<i64>) -> Result<usize, AppError>;
    async fn stream_complete(
        &self,
        input: CanvasTextGenerationLlmCall,
    ) -> Result<ChatResponse, AppError>;
}

#[async_trait]
pub trait CanvasPreviewApplicationPort: Send + Sync {
    async fn prepare_node(
        &self,
        input: CanvasPreviewNodeApplicationInput,
    ) -> Result<CanvasNodeRequestPreview, AppError>;
}

#[async_trait]
pub trait CanvasLlmTextNodeApplicationPort: Send + Sync {
    async fn run_llm_text(
        &self,
        input: CanvasLlmTextNodeApplicationInput,
        llm_port: &dyn CanvasTextGenerationLlmPort,
    ) -> Result<CanvasRunSummary, AppError>;
}

#[async_trait]
pub trait CanvasImageNodeApplicationPort: Send + Sync {
    async fn run_image(
        &self,
        input: CanvasImageNodeApplicationInput,
        host_port: &dyn ImageGenerationHostPort,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<CanvasRunSummary, AppError>;
}

#[derive(Debug, Clone)]
pub struct CanvasVideoProviderInput {
    pub user_id: i64,
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub config: VideoProviderConfig,
    pub request: a4n_video_sidecar_contract::VideoGenerationRequest,
}

pub struct CanvasVideoFileInput<'a> {
    pub project_id: i64,
    pub video_id: i64,
    pub bytes: &'a [u8],
    pub format: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasVideoPersistedFile {
    pub relative_path: String,
    pub file_size: i64,
}

#[async_trait]
pub trait CanvasVideoGenerationHostPort: CanvasVideoPreviewHostPort + Send + Sync {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError>;

    async fn resolve_canvas_video_images(
        &self,
        input: CanvasVideoImageInput,
    ) -> Result<CanvasVideoResolvedImages, AppError>;

    async fn submit_canvas_video(
        &self,
        input: CanvasVideoProviderInput,
    ) -> Result<VideoGenerationResult, AppError>;

    async fn poll_canvas_video(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        config: &VideoProviderConfig,
        job_id: &str,
    ) -> Result<VideoJobState, AppError>;

    fn persist_canvas_video_file(
        &self,
        input: CanvasVideoFileInput<'_>,
    ) -> Result<CanvasVideoPersistedFile, AppError>;

    async fn remove_canvas_video_file(&self, relative_path: &str) -> Result<(), AppError>;

    fn runtime_config_snapshot(&self) -> Option<Value>;
}

#[async_trait]
pub trait CanvasVideoNodeApplicationPort: Send + Sync {
    async fn run_video(
        &self,
        input: CanvasVideoNodeApplicationInput,
        host_port: &dyn CanvasVideoGenerationHostPort,
    ) -> Result<CanvasRunSummary, AppError>;
}
