mod api;
mod preparation;

pub use a4n_api_contract::rag::{
    EnhancedSearchInput, RagEnhancedQuerySummary, RagEnhancedSearchOutput,
    RagEnhancedSearchWithVectorsInput, RagPreparedEnhancedSearchOutput, RagSearchResult,
};
pub use api::{rag_enhanced_search, rag_enhanced_search_with_vectors, rag_prepare_enhanced_search};

pub(crate) use preparation::current_embedding_identity;
