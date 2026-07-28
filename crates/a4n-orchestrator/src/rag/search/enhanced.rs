mod contracts;
mod execution;
mod expansion;
mod fusion;
mod rules;
mod scope;

pub use contracts::{EnhancedRagQuery, EnhancedRagSearchResult, PreparedEnhancedRagSearch};
pub(crate) use execution::{
    execute_prepared_enhanced_rag_search_in_store,
    execute_prepared_enhanced_rag_search_with_vectors_in_store,
};
pub use fusion::fuse_enhanced_rag_results;
pub use rules::{
    MAX_RAG_TOP_K, enhanced_rag_pull_k, normalize_search_data_types, required_nonblank,
    required_top_k, validate_embedding_identity_match, validate_query_vectors,
};

#[cfg(test)]
mod tests;
