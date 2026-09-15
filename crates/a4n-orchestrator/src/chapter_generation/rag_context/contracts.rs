use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_rag_core::vector_store::SearchFilter;

use super::query::GenerationRagQuery;

pub(crate) struct PreparedGenerationRagSearch {
    pub(crate) project_id: i64,
    pub(crate) chapter_number: i32,
    pub(crate) queries: Vec<GenerationRagQuery>,
    pub(crate) filter: SearchFilter,
    pub(crate) embedding_identity: EmbeddingModelIdentity,
}
