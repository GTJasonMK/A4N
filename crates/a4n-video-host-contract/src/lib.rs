//! Runtime and Orchestrator ports for video provider effects.
//!
//! Workflow plans and persisted execution contracts stay in
//! `a4n-video-contract`; this crate contains only the Host-facing adapters
//! needed while preparing, submitting, polling, and persisting video output.

use a4n_domain::media::GeneratedMediaArtifactFile;
use a4n_error::AppError;
use a4n_generation_artifact_contract::GenerationArtifactRef;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video_provider_contract::{
    ProviderImageInputTransport, VideoGenerationResult, VideoJobState,
};
use a4n_video_sidecar_contract::VideoGenerationRequest;
use async_trait::async_trait;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

/// Minimal target returned before provider-job polling. The video owner
/// decides whether a job is active; Runtime only resolves the selected config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoProviderPollTarget {
    Inactive,
    Active { config_id: Option<i64> },
}

/// Media facts discovered by a Host probe before the video owner persists a
/// generated or imported record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImportedVideoProperties {
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct VideoProviderRequestInput {
    pub project_id: i64,
    pub config: VideoProviderConfig,
    pub request: VideoGenerationRequest,
    pub provider_idempotency_key_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VideoProviderRequestBody {
    pub request: serde_json::Value,
    pub source_label: String,
}

/// Request validation mode shared by preview and generation preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoProviderValidationMode {
    Generate,
    Preview,
}

#[derive(Debug, Clone)]
pub struct CanvasVideoImageInput {
    pub user_id: i64,
    pub project_id: i64,
    pub mode: VideoProviderValidationMode,
    pub first_frame_image_id: Option<i64>,
    pub last_frame_image_id: Option<i64>,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct CanvasVideoResolvedImages {
    pub first_frame_image_url: Option<String>,
    pub last_frame_image_url: Option<String>,
    pub reference_image_urls: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoImageInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub mode: VideoProviderValidationMode,
    pub transport: ProviderImageInputTransport,
    pub reference_image_ids: Vec<i64>,
    pub first_frame_image_id: Option<i64>,
    pub last_frame_image_id: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedChapterVideoImageInputs {
    pub reference_image_urls: Vec<String>,
    pub first_frame_image_url: Option<String>,
    pub last_frame_image_url: Option<String>,
    pub staging_artifacts: Vec<GenerationArtifactRef>,
}

#[derive(Debug, Clone)]
pub struct CanvasVideoRequestPreview {
    pub config_id: i64,
    pub provider_backend: String,
    pub model: String,
    pub graph_id: i64,
    pub node_id: i64,
    pub request: VideoGenerationRequest,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoRequestPreview {
    pub request: serde_json::Value,
    pub request_kind: &'static str,
    pub request_title: &'static str,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct ReferenceMediaVideoRequestPreview {
    pub request: serde_json::Value,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct PersistedVideoResult {
    pub video_id: i64,
    pub project_id: i64,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub file_size: i64,
    pub metadata_json: Option<serde_json::Value>,
}

/// Minimal video facts needed by the Runtime media adapter. The provider host
/// must not depend on the generated-video ORM entity.
#[derive(Debug, Clone)]
pub struct VideoProviderResultTarget {
    pub video_id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
}

pub type VideoCancellationFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Runtime-neutral cancellation bridge used by the video provider host port.
///
/// Runtime and Orchestrator adapt their local cancellation implementation at
/// the boundary; the contract crate stays free of Tokio dependencies.
#[derive(Clone)]
pub struct VideoCancellationHandle {
    cancelled: Arc<dyn Fn() -> VideoCancellationFuture + Send + Sync>,
}

impl VideoCancellationHandle {
    pub fn new(cancelled: impl Fn() -> VideoCancellationFuture + Send + Sync + 'static) -> Self {
        Self {
            cancelled: Arc::new(cancelled),
        }
    }

    pub fn cancelled(&self) -> VideoCancellationFuture {
        (self.cancelled)()
    }
}

/// Runtime adapter for provider execution owned by the video application.
///
/// The contract carries provider/file effects without exposing ORM entities or
/// the Orchestrator facade to Runtime.
#[async_trait]
pub trait VideoProviderHostPort: Send + Sync {
    fn inspect_video_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError>;

    async fn poll_video_job(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        config: &VideoProviderConfig,
        job_id: &str,
        cancel_token: &VideoCancellationHandle,
    ) -> Result<VideoJobState, AppError>;

    async fn persist_video_result(
        &self,
        target: &VideoProviderResultTarget,
        prompt_id: Option<i64>,
        result: VideoGenerationResult,
    ) -> Result<PersistedVideoResult, AppError>;

    async fn remove_video_result(&self, persisted: &PersistedVideoResult) -> Result<(), AppError>;
}

/// Progress and cancellation surface shared by generation Hosts.
///
/// The concrete owner may attach checkpoints and event publishing behind this
/// interface; provider adapters only need to report progress and observe the
/// cancellation bridge.
pub trait VideoGenerationProgressPort: Send {
    fn progress(&mut self, current: u32, total: u32, message: &str) -> Result<(), AppError>;

    fn cancellation_handle(&self) -> VideoCancellationHandle;

    fn cancel(&self);
}

/// Runtime adapters needed by chapter and reference-media video generation.
///
/// These traits intentionally contain only provider/file effects. Validation,
/// persistence, and generation task state remain in the video owner.
#[async_trait]
pub trait ChapterVideoGenerationHostPort: ChapterVideoPreviewHostPort {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError>;

    fn resolve_image_transport(&self) -> Result<ProviderImageInputTransport, AppError>;

    async fn resolve_chapter_video_image_inputs(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError>;

    async fn chapter_video_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError>;

    async fn submit_chapter_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError>;

    fn runtime_config_snapshot(&self) -> Option<serde_json::Value>;

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort;
}

#[async_trait]
pub trait ReferenceMediaVideoGenerationHostPort: ReferenceMediaVideoPreviewHostPort {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError>;

    async fn reference_media_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError>;

    async fn submit_reference_media_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError>;

    async fn upload_reference_media_input(
        &self,
        owner_user_id: i64,
        project_id: i64,
        source_media_id: i64,
        source_media_kind: &str,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(String, GenerationArtifactRef), AppError> {
        let _ = (
            owner_user_id,
            project_id,
            source_media_id,
            source_media_kind,
            file_name,
            mime_type,
            bytes,
        );
        Err(AppError::InvalidArgument(
            "当前运行环境不支持本地参考媒体临时上传".to_string(),
        ))
    }

    fn runtime_config_snapshot(&self) -> Option<serde_json::Value>;

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort;
}

pub trait VideoGenerationHostPort:
    ChapterVideoGenerationHostPort + ReferenceMediaVideoGenerationHostPort
{
}

impl<T> VideoGenerationHostPort for T where
    T: ChapterVideoGenerationHostPort + ReferenceMediaVideoGenerationHostPort
{
}

/// Minimal prompt-worker Host. The owner adapts this renderer to its internal
/// prompt implementation without exposing that implementation in the contract.
#[async_trait]
pub trait VideoPromptGenerationHostPort: Send + Sync {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError>;

    fn runtime_video_config(&self) -> Option<VideoProviderConfig>;
}

/// Runtime adapters needed to prepare a canvas video request for preview.
#[async_trait]
pub trait CanvasVideoPreviewHostPort: Send + Sync {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError>;

    async fn resolve_canvas_video_images_for_preview(
        &self,
        input: CanvasVideoImageInput,
    ) -> Result<CanvasVideoResolvedImages, AppError>;
}

/// Runtime adapters needed to prepare a chapter video request for preview or
/// generation. Generation-only adapters remain in the Orchestrator-facing
/// host trait.
#[async_trait]
pub trait ChapterVideoPreviewHostPort: Send + Sync {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError>;

    fn resolve_image_transport_for_preview(&self) -> Result<ProviderImageInputTransport, AppError>;

    async fn resolve_chapter_video_image_inputs_for_preview(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError>;

    async fn chapter_video_provider_request_body_for_preview(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError>;
}

/// Runtime adapters needed to prepare a reference-media video request.
#[async_trait]
pub trait ReferenceMediaVideoPreviewHostPort: Send + Sync {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError>;

    async fn reference_media_provider_request_body_for_preview(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError>;

    async fn upload_reference_media_input_for_preview(
        &self,
        owner_user_id: i64,
        project_id: i64,
        source_media_id: i64,
        source_media_kind: &str,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(String, GenerationArtifactRef), AppError> {
        let _ = (
            owner_user_id,
            project_id,
            source_media_id,
            source_media_kind,
            file_name,
            mime_type,
            bytes,
        );
        Err(AppError::InvalidArgument(
            "当前运行环境不支持本地参考媒体临时上传".to_string(),
        ))
    }
}
