pub(crate) mod enhanced;
mod execution;
mod filter;

pub use enhanced::{
    EnhancedRagQuery, EnhancedRagSearchResult, PreparedEnhancedRagSearch,
    normalize_search_data_types, required_nonblank, required_top_k,
};
pub(crate) use execution::{
    hybrid_rag_search_filtered_in_store, hybrid_rag_search_with_validated_embedding_in_store,
};
pub(crate) use filter::identity_scoped_search_filter;

#[cfg(test)]
mod tests;
