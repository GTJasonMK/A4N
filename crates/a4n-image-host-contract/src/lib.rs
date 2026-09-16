//! Runtime and application-owner contracts for image generation effects.
//!
//! Provider execution, file publication and request preparation are kept
//! behind this boundary so Runtime consumers do not depend on the
//! Orchestrator media facade.

use a4n_api_contract::image_generation::{
    GenerateBatchImagesInput, GenerateGalgameImageInput, GeneratePageImageInput,
    GenerateSceneImageInput, ObjectProfileImageInput,
};
use a4n_api_contract::profile::{ProfileMaterialGroup, UploadProfileMaterialImageInput};
use a4n_canvas_contract::GenerateCanvasImageInput;
use a4n_domain::media::GeneratedMediaArtifactFile;
use a4n_error::AppError;
use a4n_image_contract::{ImageGenerationRequest, ImageReference};
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

/// Host-facing boundary for Galgame image generation.
#[async_trait]
pub trait GalgameImageGenerationApplicationPort: Send + Sync {
    async fn generate_galgame_image(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateGalgameImageInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<a4n_api_contract::image::GeneratedImageSummary, AppError>;
}

#[async_trait]
pub trait ImageObjectProfileRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_object_profile_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: ObjectProfileImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError>;
}

#[async_trait]
pub trait ImageBatchRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_batch_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateBatchImagesInput,
    ) -> Result<Vec<PreparedImageGenerationRequest>, AppError>;
}

#[async_trait]
pub trait ImagePageRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_page_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GeneratePageImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError>;
}

#[async_trait]
pub trait ImageCanvasRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_canvas_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateCanvasImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError>;
}

#[async_trait]
pub trait ImageSceneRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_scene_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateSceneImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError>;
}

/// Host-facing boundary for profile-material uploads that need image file
/// publication adapters supplied by Runtime.
#[async_trait]
pub trait ProfileMaterialUploadApplicationPort: Send + Sync {
    async fn upload(
        &self,
        user_id: i64,
        host: &dyn ImageGenerationHostPort,
        input: UploadProfileMaterialImageInput,
    ) -> Result<ProfileMaterialGroup, AppError>;
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
