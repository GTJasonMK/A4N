pub const LLM_MAX_TOKENS_SETTING_KEY: &str = a4n_db::config::LLM_MAX_TOKENS_SETTING_KEY;

#[derive(Debug, Clone)]
pub struct EffectiveLlmPreviewBudget {
    pub model_name: String,
    pub max_context_tokens: usize,
    pub max_output_tokens: usize,
    pub output_token_reserve: usize,
    pub output_token_source: String,
}
