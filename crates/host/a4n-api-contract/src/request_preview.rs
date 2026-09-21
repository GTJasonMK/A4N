use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ModelRequestPreview {
    pub workflow: String,
    pub title: String,
    pub steps: Vec<ModelRequestPreviewStep>,
    pub ready_step_count: usize,
    pub total_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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
