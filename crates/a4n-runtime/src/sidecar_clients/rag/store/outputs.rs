use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeletedOutput {
    pub(super) deleted: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoolOutput {
    pub(super) value: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CountOutput {
    pub(super) count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CollectionsOutput {
    pub(super) collections: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MetadataOutput {
    pub(super) metadata: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmptyOutput {}

#[cfg(test)]
mod tests {
    use super::{DeletedOutput, EmptyOutput};

    #[test]
    fn rag_output_wrappers_reject_unknown_fields() {
        let error = serde_json::from_value::<DeletedOutput>(serde_json::json!({
            "deleted": 1,
            "legacy_count": 1
        }))
        .expect_err("RAG output must reject unknown fields");
        assert!(error.to_string().contains("legacy_count"));

        let error = serde_json::from_value::<EmptyOutput>(serde_json::json!({
            "legacy_ok": true
        }))
        .expect_err("empty RAG output must reject unknown fields");
        assert!(error.to_string().contains("legacy_ok"));
    }
}
