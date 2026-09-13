pub use a4n_api_contract::image::GeneratedImageSummary;
pub use a4n_api_contract::image_generation::{
    GenerateBatchImagesInput, GenerateGalgameImageInput, GeneratePageImageInput,
    GenerateSceneImageInput, ObjectProfileImageInput, ObjectProfileImagePromptOutput,
    SceneProfileImageInput, SceneProfileImagePromptOutput,
};
use a4n_canvas_contract::GenerateCanvasImageInput;
use a4n_domain::media::GeneratedMediaArtifactFile;
use a4n_error::AppError;
use a4n_image::{ImageGenerationRequest, ImageReference};
use a4n_provider_contract::ImageProviderConfig;
use async_trait::async_trait;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

const IMAGE_CANCELLED_MESSAGE: &str = "图片生成已停止";

#[derive(Debug, Clone)]
pub struct PreparedImageGenerationRequest {
    pub project_id: i64,
    pub image_type: String,
    pub prompt_name: Option<String>,
    pub subject_name: Option<String>,
    pub preview_notes: Vec<String>,
    pub config: ImageProviderConfig,
    pub request: ImageGenerationRequest,
}

#[derive(Debug, Clone)]
pub enum ImageGenerationWorkflowBinding {
    Owned,
    Attached {
        workflow_run_id: i64,
    },
    ExistingItem {
        workflow_run_id: i64,
        step_key: String,
        step_order: i32,
        fingerprint_json: Value,
    },
}

#[derive(Debug, Clone)]
pub struct ImageGenerationExecutionInput {
    pub requested_by_user_id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub image_type: String,
    pub command: String,
    pub file_extra: Option<String>,
    pub file_context: String,
    pub config: ImageProviderConfig,
    pub request: ImageGenerationRequest,
    pub width: i32,
    pub height: i32,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub runtime_config_snapshot: Option<Value>,
    pub workflow: ImageGenerationWorkflowBinding,
    pub post_commit: ImageGenerationPostCommitAction,
}

#[derive(Debug, Clone, Default)]
pub enum ImageGenerationPostCommitAction {
    #[default]
    None,
    MangaPage {
        page_prompt_id: i64,
        page_number: i32,
    },
}

#[derive(Debug, Clone)]
pub struct ImageGenerationExecutionOutcome {
    pub summary: GeneratedImageSummary,
    pub restored: bool,
    pub created_at: chrono::NaiveDateTime,
}

pub struct ImageGenerationProgress<'a> {
    reporter: Option<&'a (dyn Fn(u32, u32, &str) + Send + Sync + 'a)>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ImageGenerationProgress<'a> {
    pub const fn none() -> Self {
        Self {
            reporter: None,
            cancel_token: None,
        }
    }

    pub fn new(
        reporter: Option<&'a (dyn Fn(u32, u32, &str) + Send + Sync + 'a)>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    pub fn progress(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    pub fn check_cancelled(&self) -> Result<(), String> {
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(IMAGE_CANCELLED_MESSAGE.to_string());
        }
        Ok(())
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.cloned().unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct ImageGenerationProviderInput {
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub config: ImageProviderConfig,
    pub request: ImageGenerationRequest,
}

#[derive(Debug, Clone)]
pub struct ImageGenerationProviderOutput {
    pub bytes: Vec<u8>,
    pub format: String,
    pub model: String,
}

pub struct PrepareGeneratedImageFileInput<'a> {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub image_type: &'a str,
    pub file_extra: Option<&'a str>,
    pub format: &'a str,
    pub publication_id: &'a str,
    pub file_context: &'a str,
}

#[derive(Debug, Clone)]
pub struct PreparedGeneratedImageFile {
    pub staging_path: String,
    pub final_path: String,
}

pub struct PersistGeneratedImageFileInput<'a> {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub image_type: &'a str,
    pub file_extra: Option<&'a str>,
    pub bytes: &'a [u8],
    pub format: &'a str,
    pub file_context: &'a str,
}

#[derive(Debug, Clone)]
pub struct PersistedGeneratedImageFile {
    pub relative_path: String,
    pub file_size: i64,
}

/// Host-facing boundary for image generation request preparation and direct
/// generation.
#[async_trait]
pub trait ImageGenerationApplicationPort: Send + Sync {
    async fn prepare_object_profile_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: ObjectProfileImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError>;

    async fn generate_galgame_image(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateGalgameImageInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<GeneratedImageSummary, AppError>;
}

/// Host-facing boundary for batch image request previews.
#[async_trait]
pub trait ImageBatchRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_batch_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateBatchImagesInput,
    ) -> Result<Vec<PreparedImageGenerationRequest>, AppError>;
}

/// Host-facing boundary for manga page image request previews.
#[async_trait]
pub trait ImagePageRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_page_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GeneratePageImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError>;
}

/// Host-facing boundary for canvas image request previews.
#[async_trait]
pub trait ImageCanvasRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_canvas_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateCanvasImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError>;
}

/// Host-facing boundary for scene image request previews.
#[async_trait]
pub trait ImageSceneRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_scene_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateSceneImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError>;
}

/// Host-facing boundary for queued image generation task ownership.
#[async_trait]
pub trait ImageGenerationTaskApplicationPort: Send + Sync {
    async fn start_task(
        &self,
        user_id: i64,
        input: &a4n_api_contract::image_generation::StartImageGenerationInput,
    ) -> Result<crate::GenerationTaskStart, crate::GenerationTaskAccessError>;

    async fn run_task(
        &self,
        user_id: i64,
        input: a4n_api_contract::image_generation::StartImageGenerationInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
        host_port: &dyn ImageGenerationHostPort,
    ) -> Result<(), String>;
}

/// Host-facing boundary for profile image prompt generation.
#[async_trait]
pub trait ImageProfilePromptApplicationPort: Send + Sync {
    async fn object_profile_prompt(
        &self,
        user_id: i64,
        input: &ObjectProfileImageInput,
    ) -> Result<ObjectProfileImagePromptOutput, AppError>;

    async fn scene_profile_prompt(
        &self,
        user_id: i64,
        input: &SceneProfileImageInput,
    ) -> Result<SceneProfileImagePromptOutput, AppError>;
}

#[async_trait]
pub trait ImageGenerationHostPort: Send + Sync {
    async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<ImageProviderConfig, AppError>;

    fn runtime_config_snapshot(&self) -> Option<Value>;

    fn read_reference_image(
        &self,
        label: &str,
        relative_path: &str,
    ) -> Result<ImageReference, AppError>;

    async fn generate_image(
        &self,
        input: ImageGenerationProviderInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<ImageGenerationProviderOutput, AppError>;

    fn prepare_generated_image_file(
        &self,
        input: PrepareGeneratedImageFileInput<'_>,
    ) -> Result<PreparedGeneratedImageFile, AppError>;

    fn persist_generated_image_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
        prepared: &PreparedGeneratedImageFile,
    ) -> Result<PersistedGeneratedImageFile, AppError>;

    fn validate_generated_image_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<(), AppError>;

    fn inspect_generated_image_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError>;

    async fn remove_generated_image_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<(), AppError>;
}
