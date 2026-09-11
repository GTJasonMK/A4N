use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct MangaPromptOptimizationInput {
    pub page_prompt: chapter_manga_prompt::Model,
    pub instruction: String,
    pub downstream_image_ids: Vec<i64>,
}

#[async_trait]
pub trait MangaPromptDownstreamCleanupPort: Send + Sync {
    async fn delete_downstream_images(&self, image_ids: &[i64]) -> Result<(), AppError>;
}

#[derive(Debug)]
pub(super) struct RequiredMangaPromptOptimizationInput {
    pub(super) page_prompt: chapter_manga_prompt::Model,
    pub(super) instruction: String,
    pub(super) downstream_image_ids: Vec<i64>,
}

pub(super) struct PreparedMangaPromptOptimization {
    pub(super) page_prompt: chapter_manga_prompt::Model,
    pub(super) downstream_image_ids: Vec<i64>,
    pub(super) request: ChatRequest,
}
