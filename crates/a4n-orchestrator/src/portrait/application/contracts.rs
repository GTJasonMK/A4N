use a4n_core::error::AppError;
use async_trait::async_trait;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

pub use a4n_api_contract::portrait::{
    GenerateAgeVariantInput, GenerateMissingPortraitsInput, GeneratePortraitInput,
    GeneratePortraitPromptInput, PortraitSummary, PortraitType, RegeneratePortraitInput,
    UploadPortraitInput,
};

pub type PortraitProgress<'a> = &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

#[derive(Debug, Clone)]
pub struct PortraitReferenceImageSource {
    pub label: String,
    pub file_path: String,
}

#[derive(Debug, Clone)]
pub struct PortraitImageProviderInput {
    pub project_id: i64,
    pub prompt: String,
    pub reference_images: Vec<PortraitReferenceImageSource>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug)]
pub struct GeneratedPortraitImage {
    pub bytes: Vec<u8>,
    pub format: String,
    pub model: String,
    pub provider: String,
    pub request_json: Value,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug)]
pub struct DecodedPortraitImage {
    pub bytes: Vec<u8>,
    pub format: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
}

pub struct PersistPortraitFileInput<'a> {
    pub project_id: i64,
    pub profile_id: i64,
    pub bytes: &'a [u8],
    pub format: &'a str,
    pub path_context: &'a str,
}

#[derive(Debug)]
pub struct PersistedPortraitFile {
    pub relative_path: String,
    pub file_size: i64,
}

#[derive(Debug, Clone)]
pub struct PortraitArtifactFile {
    pub content_type: String,
    pub sha256: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone)]
pub struct PortraitBatchItem {
    pub item_key: String,
    pub item_index: u32,
    pub summary: PortraitSummary,
}

#[derive(Debug, Clone)]
pub struct PortraitBatchOutcome {
    pub total: u32,
    pub items: Vec<PortraitBatchItem>,
    pub failed_item_keys: Vec<String>,
    pub cancelled: u32,
    pub skipped: u32,
    pub interrupted: bool,
    pub first_error: Option<String>,
}

impl PortraitBatchOutcome {
    pub fn succeeded(&self) -> u32 {
        self.items.len() as u32
    }

    pub fn failed(&self) -> u32 {
        self.failed_item_keys.len() as u32
    }
}

#[async_trait]
pub trait PortraitGenerationHostPort: Send + Sync {
    async fn generate_image(
        &self,
        input: PortraitImageProviderInput,
        cancel_token: CancellationToken,
    ) -> Result<GeneratedPortraitImage, AppError>;

    fn decode_uploaded_image(
        &self,
        file_name: &str,
        mime_type: &str,
        data_base64: &str,
    ) -> Result<DecodedPortraitImage, AppError>;

    fn persist_portrait_file(
        &self,
        input: PersistPortraitFileInput<'_>,
    ) -> Result<PersistedPortraitFile, AppError>;

    fn inspect_portrait_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<PortraitArtifactFile, AppError>;

    fn validate_portrait_file(&self, relative_path: &str, label: &str) -> Result<(), AppError>;

    async fn remove_portrait_file(&self, relative_path: &str, label: &str) -> Result<(), AppError>;
}
