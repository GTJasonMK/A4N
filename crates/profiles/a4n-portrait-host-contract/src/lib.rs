//! Runtime-facing contracts for portrait generation and upload adapters.
//!
//! Portrait validation, generation task state, permissions and persistence
//! remain with the portrait owner. Runtime supplies provider/file adapters and
//! prompt rendering only.

use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStart, GenerationTaskStartError},
    portrait::{
        GeneratePortraitInput, GeneratePortraitPromptInput, PortraitReferenceImageSource,
        PortraitSummary, StartPortraitGenerationInput, UploadPortraitInput,
    },
};
use a4n_domain::media::GeneratedMediaArtifactFile;
use a4n_error::AppError;
use a4n_image_host_contract::{
    PersistGeneratedImageFileInput, PersistedGeneratedImageFile, PrepareGeneratedImageFileInput,
    PreparedGeneratedImageFile,
};
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt_contract::rendering::PromptRenderer;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct PortraitImageProviderInput {
    pub project_id: i64,
    pub prompt: String,
    pub width: i32,
    pub height: i32,
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

    fn prepare_portrait_file(
        &self,
        input: PrepareGeneratedImageFileInput<'_>,
    ) -> Result<PreparedGeneratedImageFile, AppError>;

    fn persist_portrait_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
        prepared: &PreparedGeneratedImageFile,
    ) -> Result<PersistedGeneratedImageFile, AppError>;

    fn inspect_portrait_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError>;

    fn validate_portrait_file(&self, relative_path: &str, label: &str) -> Result<(), AppError>;

    async fn remove_portrait_file(&self, relative_path: &str, label: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait PortraitPromptRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_portrait_prompt_request(
        &self,
        input: &GeneratePortraitPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError>;
}

#[async_trait]
pub trait PortraitImageRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_portrait_request(
        &self,
        input: &GeneratePortraitInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

#[async_trait]
pub trait PortraitPromptApplicationPort: Send + Sync {
    async fn generate_portrait_prompt(
        &self,
        input: GeneratePortraitPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<String, AppError>;
}

#[async_trait]
pub trait PortraitUploadApplicationPort: Send + Sync {
    async fn upload_portrait(
        &self,
        user_id: i64,
        input: UploadPortraitInput,
        host_port: &dyn PortraitGenerationHostPort,
    ) -> Result<PortraitSummary, AppError>;
}

#[async_trait]
pub trait PortraitGenerationApplicationPort: Send + Sync {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartPortraitGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_task(
        &self,
        user_id: i64,
        input: StartPortraitGenerationInput,
        handle: GenerationTaskHandle,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn PortraitGenerationHostPort,
    ) -> Result<(), String>;
}
