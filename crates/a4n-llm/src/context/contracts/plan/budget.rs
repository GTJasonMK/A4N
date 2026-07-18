use super::super::LlmContextPackRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannedContextPack {
    pub pack_ref: LlmContextPackRef,
    pub stable_prefix: bool,
    #[serde(skip)]
    pub canonical_content_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextPlan {
    pub model_context_tokens: usize,
    pub output_token_reserve: usize,
    pub safety_margin_tokens: usize,
    pub input_utilization_percent: u8,
    pub raw_available_input_tokens: usize,
    pub available_input_tokens: usize,
    pub stable_prefix_tokens: usize,
    pub dynamic_delta_tokens: usize,
    pub selected_compression_level: String,
    pub selected_pack_refs: Vec<LlmContextPackRef>,
    #[serde(skip)]
    pub selected_packs: Vec<PlannedContextPack>,
    pub dropped_optional_pack_refs: Vec<LlmContextPackRef>,
    pub estimated_input_tokens: usize,
}
