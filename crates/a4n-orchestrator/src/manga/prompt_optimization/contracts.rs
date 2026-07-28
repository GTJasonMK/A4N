use a4n_core::{error::AppError, models::chapter::chapter_manga_prompt};
use a4n_llm::provider::ChatRequest;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct MangaPromptOptimizationInput {
    pub panel: chapter_manga_prompt::Model,
    pub instruction: String,
    pub downstream_image_ids: Vec<i64>,
}

#[async_trait]
pub trait MangaPromptDownstreamCleanupPort: Send + Sync {
    async fn delete_downstream_images(&self, image_ids: &[i64]) -> Result<(), AppError>;
}

#[derive(Debug)]
pub(super) struct RequiredMangaPromptOptimizationInput {
    pub(super) panel: chapter_manga_prompt::Model,
    pub(super) instruction: String,
    pub(super) downstream_image_ids: Vec<i64>,
}

pub(super) struct PreparedMangaPromptOptimization {
    pub(super) panel: chapter_manga_prompt::Model,
    pub(super) downstream_image_ids: Vec<i64>,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
    pub(super) request: ChatRequest,
}

pub(super) type MangaPromptOptimizationResult = Result<chapter_manga_prompt::Model, AppError>;
