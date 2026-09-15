use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_llm_provider_contract::provider::ChatRequest;

#[derive(Debug, Clone)]
pub struct MangaPromptOptimizationInput {
    pub page_prompt: chapter_manga_prompt::Model,
    pub instruction: String,
    pub downstream_image_ids: Vec<i64>,
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
