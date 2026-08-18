use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::vector_store::{SearchFilter, SearchResult};

pub struct EnhancedRagQuery {
    pub label: &'static str,
    pub query: String,
    pub(crate) weight: f32,
}

pub struct PreparedEnhancedRagSearch {
    pub project_id: i64,
    pub queries: Vec<EnhancedRagQuery>,
    pub filter: SearchFilter,
    pub top_k: usize,
    pub chapter_number: Option<i32>,
    pub embedding_identity: EmbeddingModelIdentity,
}

pub struct EnhancedRagSearchResult {
    pub results: Vec<SearchResult>,
    pub queries: Vec<EnhancedRagQuery>,
    pub candidate_count: usize,
}
