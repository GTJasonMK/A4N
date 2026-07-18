use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::vector_store::{SearchFilter, SearchResult};

#[derive(Debug, Clone)]
pub struct EnhancedRagQuery {
    pub kind: &'static str,
    pub query: String,
    pub weight: f32,
}

pub struct PreparedEnhancedRagSearch {
    pub queries: Vec<EnhancedRagQuery>,
    pub filter: SearchFilter,
    pub top_k: usize,
    pub pull_k: usize,
    pub chapter_number: Option<i32>,
    pub embedding_identity: EmbeddingModelIdentity,
}

#[derive(Debug)]
pub struct EnhancedRagSearchResult {
    pub results: Vec<SearchResult>,
    pub queries: Vec<EnhancedRagQuery>,
    pub candidate_count: usize,
}
