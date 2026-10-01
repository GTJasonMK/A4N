use super::*;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video_host_contract::VideoProviderPollTarget;

/// Host-facing lookup boundary for persisted video provider configuration.
///
/// Runtime may apply a local snapshot or user-scope policy before falling back
/// to this owner; the implementation remains free of runtime state and ORM
/// types.
#[async_trait::async_trait]
pub trait VideoProviderConfigPort: Send + Sync {
    async fn by_id(&self, config_id: i64) -> Result<VideoProviderConfig, a4n_error::AppError>;

    async fn default(&self) -> Result<VideoProviderConfig, a4n_error::AppError>;

    async fn for_provider_job(
        &self,
        project_id: Option<i64>,
        provider_kind: &str,
        provider_job_id: &str,
    ) -> Result<VideoProviderConfig, a4n_error::AppError>;
}

/// Host-facing video owner boundary for provider-job recovery.
///
/// The video owner keeps permission checks and status transitions; Runtime
/// supplies only the provider execution adapter from `a4n-video-host-contract`.
#[async_trait::async_trait]
pub trait VideoProviderExecutionApplicationPort: Send + Sync {
    async fn poll_target(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<VideoProviderPollTarget, a4n_error::AppError>;

    async fn poll_provider_job(
        &self,
        user_id: i64,
        video_id: i64,
        config: Option<&VideoProviderConfig>,
        host_port: &dyn a4n_video_host_contract::VideoProviderHostPort,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;
}

/// Input shared by chapter video request preview and generation.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub prompt_id: i64,
    #[serde(default)]
    pub config_id: Option<i64>,
}

/// Input shared by reference-media video request preview and generation.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReferenceMediaVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub input: ReferenceMediaVideoInput,
}

/// Host-facing request preview boundary for canvas video nodes.
#[async_trait::async_trait]
pub trait CanvasVideoRequestPreviewApplicationPort: Send + Sync {
    async fn preview_canvas(
        &self,
        input: crate::canvas::GenerateCanvasVideoInput,
        host_port: &dyn a4n_video_host_contract::CanvasVideoPreviewHostPort,
    ) -> Result<a4n_video_host_contract::CanvasVideoRequestPreview, a4n_error::AppError>;
}

/// Host-facing request preview boundary for chapter videos.
#[async_trait::async_trait]
pub trait ChapterVideoRequestPreviewApplicationPort: Send + Sync {
    async fn preview_chapter(
        &self,
        host_port: &dyn a4n_video_host_contract::ChapterVideoPreviewHostPort,
        input: GenerateChapterVideoInput,
        prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
    ) -> Result<a4n_video_host_contract::ChapterVideoRequestPreview, a4n_error::AppError>;
}

/// Host-facing request preview boundary for reference-media videos.
#[async_trait::async_trait]
pub trait ReferenceMediaVideoRequestPreviewApplicationPort: Send + Sync {
    async fn preview_reference_media(
        &self,
        host_port: &dyn a4n_video_host_contract::ReferenceMediaVideoPreviewHostPort,
        input: GenerateReferenceMediaVideoInput,
    ) -> Result<a4n_video_host_contract::ReferenceMediaVideoRequestPreview, a4n_error::AppError>;
}

/// Host-facing boundary for the video generation worker.
///
/// Runtime owns scheduling and supplies provider/prompt adapters. Reservation,
/// validation, checkpointing, and result commits remain in the video owner.
#[async_trait::async_trait]
pub trait VideoGenerationApplicationPort: Send + Sync {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartVideoGenerationInput,
    ) -> Result<
        crate::generation_tasks::GenerationTaskStart,
        crate::generation_tasks::GenerationTaskStartError,
    >;

    async fn run_task(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
        handle: crate::generation_tasks::GenerationTaskHandle,
        host_port: &dyn a4n_video_host_contract::VideoGenerationHostPort,
    ) -> Result<(), String>;

    async fn run_prompt_task(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
        handle: crate::generation_tasks::GenerationTaskHandle,
        host_port: &dyn a4n_video_host_contract::VideoPromptGenerationHostPort,
    ) -> Result<(), String>;
}
