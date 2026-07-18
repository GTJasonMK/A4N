use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::vector_store::SearchFilter;
use serde::Serialize;

use super::query::GenerationRagQueries;

pub struct PreparedGenerationRagSearch {
    pub project_id: i64,
    pub chapter_number: i32,
    pub queries: GenerationRagQueries,
    pub filter: SearchFilter,
    pub embedding_identity: EmbeddingModelIdentity,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GenerationRagContext {
    pub text: Option<String>,
    pub statistics: GenerationRagStatistics,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GenerationRagStatistics {
    pub chunk_count: usize,
    pub summary_count: usize,
    pub continuity_count: usize,
    pub context_length: usize,
    pub query_main: Option<String>,
    pub query_characters: Vec<String>,
    pub query_foreshadowing: Vec<String>,
    pub query_location: Option<String>,
}
