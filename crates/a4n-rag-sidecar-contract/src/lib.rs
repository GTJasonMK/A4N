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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DropCollectionPayload {
    pub collection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompletenessByIdsPayload {
    pub collection: String,
    pub expected_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompletenessReport {
    pub collection: String,
    pub expected: usize,
    pub actual_vector: usize,
    pub actual_bm25: usize,
    pub is_complete: bool,
    pub missing_vector_ids: Vec<String>,
    pub missing_bm25_ids: Vec<String>,
    pub missing_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        CompletenessByIdsPayload, CompletenessReport, DropCollectionPayload, MetadataByIdsPayload,
    };

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

    #[test]
    fn drop_collection_contract_rejects_unknown_fields() {
        let error = serde_json::from_value::<DropCollectionPayload>(serde_json::json!({
            "collection": "42",
            "create_if_missing": true
        }))
        .expect_err("unknown collection fields must fail at the sidecar boundary");
        assert!(error.to_string().contains("create_if_missing"));
    }

    #[test]
    fn completeness_contract_rejects_unknown_fields() {
        let error = serde_json::from_value::<CompletenessByIdsPayload>(serde_json::json!({
            "collection": "42",
            "expected_ids": [],
            "include_bm25": true
        }))
        .expect_err("unknown completeness fields must fail at the sidecar boundary");
        assert!(error.to_string().contains("include_bm25"));

        let error = serde_json::from_value::<CompletenessReport>(serde_json::json!({
            "collection": "42",
            "expected": 0,
            "actual_vector": 0,
            "actual_bm25": 0,
            "is_complete": true,
            "missing_vector_ids": [],
            "missing_bm25_ids": [],
            "missing_ids": [],
            "legacy_field": true
        }))
        .expect_err("unknown completeness output fields must fail at the sidecar boundary");
        assert!(error.to_string().contains("legacy_field"));
    }
}
