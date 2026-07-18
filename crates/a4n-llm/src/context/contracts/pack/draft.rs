use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextPackDraft {
    pub project_id: i64,
    pub root_project_id: i64,
    pub scope_kind: String,
    pub workflow: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_hash: String,
    pub canonical_content_json: Value,
    pub canonical_content_text: String,
    pub content_sha256: String,
    pub compression_level: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone)]
pub struct LlmContextPackDraftInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub scope_kind: String,
    pub workflow: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_hash: String,
    pub compression_level: String,
    pub canonical_content_json: Value,
}
