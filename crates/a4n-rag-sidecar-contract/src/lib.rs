//! Serialized contracts for the RAG sidecar.
//!
//! The storage engine and host-side vector-store abstraction stay outside this
//! crate so the capability client can depend on the wire shape alone.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MetadataByIdsPayload {
    pub collection: String,
    pub ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::MetadataByIdsPayload;

    #[test]
    fn metadata_by_ids_rejects_unknown_fields() {
        let error = serde_json::from_value::<MetadataByIdsPayload>(serde_json::json!({
            "collection": "42",
            "ids": [],
            "include_embeddings": true
        }))
        .expect_err("unknown metadata fields must fail at the sidecar boundary");
        assert!(error.to_string().contains("include_embeddings"));
    }
}
