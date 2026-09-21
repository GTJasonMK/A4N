use serde_json::Value;

#[derive(Debug, Clone)]
pub struct LlmContextLedgerPackDraft {
    pub compression_level: String,
    pub canonical_content_json: Value,
}

#[derive(Debug, Clone)]
pub struct LlmContextLedgerPackInput {
    pub scope_kind: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_identity_json: Value,
    pub required: bool,
    pub stable_prefix: bool,
    pub variants: Vec<LlmContextLedgerPackDraft>,
}
