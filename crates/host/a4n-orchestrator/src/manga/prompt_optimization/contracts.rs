use a4n_domain::manga::MangaPagePromptRecord;
use a4n_llm_provider_contract::provider::ChatRequest;

#[derive(Debug, Clone)]
pub struct MangaPromptOptimizationInput {
    pub page_prompt: MangaPagePromptRecord,
    pub instruction: String,
    pub downstream_image_ids: Vec<i64>,
}

#[derive(Debug)]
pub(super) struct RequiredMangaPromptOptimizationInput {
    pub(super) page_prompt: MangaPagePromptRecord,
    pub(super) instruction: String,
    pub(super) downstream_image_ids: Vec<i64>,
}

pub(super) struct PreparedMangaPromptOptimization {
    pub(super) page_prompt: MangaPagePromptRecord,
    pub(super) downstream_image_ids: Vec<i64>,
    pub(super) request: ChatRequest,
}
