use a4n_llm::provider::ChatRequest;

pub use a4n_api_contract::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};

pub(crate) struct ChatRequestStepInput {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) prompt_name: Option<String>,
    pub(crate) state: String,
    pub(crate) note: Option<String>,
    pub(crate) request: ChatRequest,
}

pub(crate) struct ChatRequestStepWithBudgetInput {
    pub(crate) step: ChatRequestStepInput,
    pub(crate) model_context_tokens: usize,
    pub(crate) output_token_reserve: usize,
    pub(crate) budget_note: Option<String>,
}
