use a4n_domain::manga::MangaPagePromptRecord;
use a4n_llm_provider_contract::provider::ChatRequest;

#[derive(Debug, Clone)]
pub struct MangaPromptOptimizationInput {
    pub page_prompt: MangaPagePromptRecord,
    pub instruction: String,
    pub expected_updated_at: String,
}

#[derive(Debug)]
pub(super) struct RequiredMangaPromptOptimizationInput {
    pub(super) page_prompt: MangaPagePromptRecord,
    pub(super) instruction: String,
}

pub(super) struct PreparedMangaPromptOptimization {
    pub(super) page_prompt: MangaPagePromptRecord,
    pub(super) request: ChatRequest,
}
