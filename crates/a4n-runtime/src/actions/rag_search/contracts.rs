use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_orchestrator::rag::EnhancedRagQuery;
use a4n_rag_core::vector_store::SearchResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub vector_score: Option<f32>,
    pub bm25_score: Option<f32>,
    pub metadata: serde_json::Value,
}

impl From<SearchResult> for RagSearchResult {
    fn from(result: SearchResult) -> Self {
        Self {
            id: result.id,
            content: result.content,
            score: result.score,
            vector_score: result.vector_score,
            bm25_score: result.bm25_score,
            metadata: result.metadata,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnhancedSearchInput {
    pub query: String,
    pub top_k: Option<usize>,
    pub chapter_number: Option<i32>,
    pub data_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagEnhancedQuerySummary {
    pub kind: String,
    pub query: String,
    pub weight: f32,
}

impl From<&EnhancedRagQuery> for RagEnhancedQuerySummary {
    fn from(query: &EnhancedRagQuery) -> Self {
        Self {
            kind: query.kind.to_string(),
            query: query.query.clone(),
            weight: query.weight,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagEnhancedSearchOutput {
    pub results: Vec<RagSearchResult>,
    pub queries: Vec<RagEnhancedQuerySummary>,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagPreparedEnhancedSearchOutput {
    pub queries: Vec<RagEnhancedQuerySummary>,
    pub query_texts: Vec<String>,
    pub embedding_identity: EmbeddingModelIdentity,
    pub top_k: usize,
    pub pull_k: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RagEnhancedSearchWithVectorsInput {
    pub search: EnhancedSearchInput,
    pub embedding_identity: EmbeddingModelIdentity,
    pub query_vectors: Vec<Vec<f32>>,
}
