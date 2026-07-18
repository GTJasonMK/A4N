use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextRequestPreview {
    pub manifest_json: Value,
    pub plan_json: Value,
    pub pack_refs_json: Value,
    pub stable_prefix_tokens: usize,
    pub dynamic_delta_tokens: usize,
    pub available_input_tokens: usize,
}
