pub use a4n_api_contract::{
    image::GeneratedImageSummary,
    image_generation::{
        GenerateBatchImagesInput, GeneratePageImageInput, GenerateSceneImageInput,
        ObjectProfileImageInput, ObjectProfileImagePromptOutput, SceneProfileImageInput,
        SceneProfileImagePromptOutput,
    },
};
use a4n_core::{error::AppError, models::ai::image_gen_config};
use a4n_image::{ImageGenerationRequest, ImageReference};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::generation::GeneratedMediaArtifactFile;

const IMAGE_CANCELLED_MESSAGE: &str = "图片生成已停止";

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
    #[serde(default)]
    pub source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateGalgameImageInput {
    pub project_id: i64,
    pub prompt: String,
    pub image_type: String,
    pub config_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub generation_spec_id: i64,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: i64,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PreparedImageGenerationRequest {
    pub project_id: i64,
    pub image_type: String,
    pub prompt_name: Option<String>,
    pub subject_name: Option<String>,
    pub preview_notes: Vec<String>,
    pub config: image_gen_config::Model,
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
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub image_type: String,
    pub command: String,
    pub file_extra: Option<String>,
    pub file_context: String,
    pub config: image_gen_config::Model,
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
    pub config: image_gen_config::Model,
    pub request: ImageGenerationRequest,
}

#[derive(Debug, Clone)]
pub struct ImageGenerationProviderOutput {
    pub bytes: Vec<u8>,
    pub format: String,
    pub model: String,
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

#[async_trait]
pub trait ImageGenerationHostPort: Send + Sync {
    async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError>;

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

    fn persist_generated_image_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
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
