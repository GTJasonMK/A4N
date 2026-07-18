use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct ModelRequestPreview {
    pub workflow: String,
    pub title: String,
    pub steps: Vec<ModelRequestPreviewStep>,
    pub ready_step_count: usize,
    pub total_chars: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelRequestPreviewStep {
    pub key: String,
    pub title: String,
    pub kind: String,
    pub prompt_name: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub system_prompt: Option<String>,
    pub user_prompt: Option<String>,
    pub messages: Option<Value>,
    pub request: Option<Value>,
    pub context_manifest: Option<Value>,
    pub context_plan: Option<Value>,
    pub context_pack_refs: Option<Value>,
    pub total_chars: usize,
    pub char_counts: Option<Value>,
}

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
    pub(crate) output_token_reserve_override: Option<usize>,
    pub(crate) budget_note: Option<String>,
}
