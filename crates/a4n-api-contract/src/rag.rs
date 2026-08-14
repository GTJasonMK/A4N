use crate::embedding::EmbeddingModelIdentity;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RagSearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub vector_score: Option<f32>,
    pub bm25_score: Option<f32>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnhancedSearchInput {
    pub query: String,
    #[schemars(range(min = 1, max = 50))]
    pub top_k: usize,
    #[schemars(range(min = 1))]
    pub chapter_number: Option<i32>,
    pub data_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RagEnhancedQuerySummary {
    pub kind: String,
    pub query: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RagEnhancedSearchOutput {
    pub results: Vec<RagSearchResult>,
    pub queries: Vec<RagEnhancedQuerySummary>,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RagPreparedEnhancedSearchOutput {
    pub queries: Vec<RagEnhancedQuerySummary>,
    pub query_texts: Vec<String>,
    pub embedding_identity: EmbeddingModelIdentity,
    pub top_k: usize,
    pub pull_k: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RagEnhancedSearchWithVectorsInput {
    pub search: EnhancedSearchInput,
    pub embedding_identity: EmbeddingModelIdentity,
    pub query_vectors: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RagIngestOptions {
    pub rebuild: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedClientRagIngestRecord {
    pub chunk_id: String,
    pub text: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClientRagIngestPrepareOutput {
    pub operation_trace_id: String,
    pub embedding_identity: EmbeddingModelIdentity,
    pub rebuild: bool,
    pub records: Vec<PreparedClientRagIngestRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UploadedRagEmbedding {
    pub chunk_id: String,
    pub content_hash: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClientRagIngestUploadInput {
    pub operation_trace_id: String,
    pub embedding_identity: EmbeddingModelIdentity,
    pub rebuild: bool,
    pub embeddings: Vec<UploadedRagEmbedding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RagIngestResult {
    pub chunk_count: u32,
    pub added_count: u32,
    pub updated_count: u32,
    pub skipped_count: u32,
    pub deleted_count: u32,
}

#[cfg(test)]
mod tests {
    use super::ClientRagIngestUploadInput;
    use serde_json::json;

    #[test]
    fn client_ingest_upload_contract_requires_hash_and_rebuild_and_rejects_unknown_fields() {
        let valid = json!({
            "operation_trace_id": "rag-ingest:1:trace",
            "embedding_identity": {
                "backend": "local",
                "model": "test",
                "dimension": 2
            },
            "rebuild": false,
            "embeddings": [{
                "chunk_id": "chapter:1",
                "content_hash": "hash",
                "embedding": [0.1, 0.2]
            }]
        });
        assert!(serde_json::from_value::<ClientRagIngestUploadInput>(valid.clone()).is_ok());

        let mut missing_rebuild = valid.clone();
        missing_rebuild.as_object_mut().unwrap().remove("rebuild");
        assert!(serde_json::from_value::<ClientRagIngestUploadInput>(missing_rebuild).is_err());

        let mut missing_hash = valid.clone();
        missing_hash["embeddings"][0]
            .as_object_mut()
            .unwrap()
            .remove("content_hash");
        assert!(serde_json::from_value::<ClientRagIngestUploadInput>(missing_hash).is_err());

        let mut unknown_field = valid;
        unknown_field["fallback"] = json!(true);
        assert!(serde_json::from_value::<ClientRagIngestUploadInput>(unknown_field).is_err());
    }
}
