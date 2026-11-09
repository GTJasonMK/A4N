pub use a4n_api_contract::image::GeneratedImageSummary;
pub use a4n_api_contract::image_generation::{
    GenerateBatchImagesInput, GenerateGalgameImageInput, GeneratePageImageInput,
    GenerateSceneImageInput, ObjectProfileImageInput, ObjectProfileImagePromptOutput,
    SceneProfileImageInput, SceneProfileImagePromptOutput,
};
use a4n_image_contract::ImageGenerationRequest;
use a4n_provider_contract::ImageProviderConfig;
use serde_json::Value;

pub(crate) use a4n_image_host_contract::{
    ImageGenerationHostPort, ImageGenerationProgress, ImageGenerationProviderInput,
    ImageGenerationProviderOutput, PersistGeneratedImageFileInput, PrepareGeneratedImageFileInput,
    PreparedImageGenerationRequest,
};

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
        expected_prompt_updated_at: chrono::NaiveDateTime,
    },
}

#[derive(Debug, Clone)]
pub struct ImageGenerationExecutionOutcome {
    pub summary: GeneratedImageSummary,
    pub restored: bool,
    pub created_at: chrono::NaiveDateTime,
}
