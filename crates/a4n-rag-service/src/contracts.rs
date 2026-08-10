use a4n_rag_core::vector_store::{DeleteFilter, EmbeddingRecord, SearchFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IngestPayload {
    pub(crate) collection: String,
    pub(crate) records: Vec<EmbeddingRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct IngestOutput {
    pub(crate) written: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchPayload {
    pub(crate) collection: String,
    pub(crate) query_text: String,
    pub(crate) query_embedding: Vec<f32>,
    pub(crate) top_k: usize,
    #[serde(default)]
    pub(crate) filters: Option<SearchFilter>,
    #[serde(default)]
    pub(crate) vector_weight: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QueryPayload {
    pub(crate) collection: String,
    pub(crate) query_embedding: Vec<f32>,
    pub(crate) top_k: usize,
    #[serde(default)]
    pub(crate) filters: Option<SearchFilter>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchTextPayload {
    pub(crate) collection: String,
    pub(crate) query_text: String,
    pub(crate) top_k: usize,
    #[serde(default)]
    pub(crate) filters: Option<SearchFilter>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeletePayload {
    pub(crate) collection: String,
    pub(crate) filter: DeleteFilter,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CollectionPayload {
    pub(crate) collection: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MetadataByIdsPayload {
    pub(crate) collection: String,
    pub(crate) ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MetadataByFilterPayload {
    pub(crate) collection: String,
    pub(crate) filter: DeleteFilter,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompletenessByIdsPayload {
    pub(crate) collection: String,
    pub(crate) expected_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeleteOutput {
    pub(crate) deleted: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MetadataOutput {
    pub(crate) metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct EmptyOutput {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rag_payload_rejects_unknown_fields() {
        let error = serde_json::from_value::<CollectionPayload>(
            serde_json::json!({"collection": "1", "create_if_missing": true}),
        )
        .expect_err("unknown RAG payload fields must fail");
        assert!(
            error
                .to_string()
                .contains("unknown field `create_if_missing`")
        );
    }

    #[test]
    fn rag_ingest_requires_string_content() {
        for record in [
            serde_json::json!({
                "id": "chapter:1:1",
                "embedding": [1.0, 0.0],
                "metadata": {"project_id": 1}
            }),
            serde_json::json!({
                "id": "chapter:1:1",
                "embedding": [1.0, 0.0],
                "metadata": {"project_id": 1},
                "content": null
            }),
        ] {
            serde_json::from_value::<IngestPayload>(serde_json::json!({
                "collection": "1",
                "records": [record]
            }))
            .expect_err("missing or null content must fail at the sidecar boundary");
        }
    }
}
