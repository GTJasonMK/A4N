use super::super::contracts::LlmContextLedgerPackDraft;
use serde_json::Value;

impl LlmContextLedgerPackDraft {
    pub fn new(compression_level: impl Into<String>, canonical_content_json: Value) -> Self {
        Self {
            compression_level: compression_level.into(),
            canonical_content_json,
        }
    }
}
