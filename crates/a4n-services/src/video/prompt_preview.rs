use serde::Serialize;
use serde_json::Value;

mod local;
mod preview_steps;

#[derive(Debug, Serialize)]
pub struct VideoPromptRequestPreview {
    pub workflow: String,
    pub title: String,
    pub steps: Vec<VideoPromptRequestPreviewStep>,
}

#[derive(Debug, Serialize)]
pub struct VideoPromptRequestPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
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
