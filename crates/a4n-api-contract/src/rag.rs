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
    pub top_k: Option<usize>,
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
