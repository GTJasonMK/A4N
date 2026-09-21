//! Runtime-facing contracts for Galgame asset generation adapters.

use a4n_api_contract::{
    galgame::{GalgameAssetGenerationSummary, RunGalgameAssetGenerationInput},
    image::GeneratedImageSummary,
    image_generation::GenerateGalgameImageInput,
    portrait::{GeneratePortraitInput, PortraitSummary},
};
use a4n_error::AppError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

#[async_trait]
pub trait GalgameAssetGenerationHostPort: Send + Sync {
    async fn render_image_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: i64,
    ) -> Result<String, AppError>;

    async fn generate_visual_image(
        &self,
        user_id: i64,
        input: GenerateGalgameImageInput,
    ) -> Result<GeneratedImageSummary, AppError>;

    async fn generate_portrait_image(
        &self,
        user_id: i64,
        input: GeneratePortraitInput,
    ) -> Result<PortraitSummary, AppError>;

    fn validate_data_file(&self, relative_path: &str, label: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait GalgameAssetGenerationApplicationPort: Send + Sync {
    async fn run(
        &self,
        host_port: &dyn GalgameAssetGenerationHostPort,
        user_id: i64,
        input: RunGalgameAssetGenerationInput,
    ) -> Result<GalgameAssetGenerationSummary, AppError>;
}
