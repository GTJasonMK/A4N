mod api;
mod preparation;

pub use api::*;
pub use a4n_api_contract::rag::{
    EnhancedSearchInput, RagEnhancedQuerySummary, RagEnhancedSearchOutput,
    RagEnhancedSearchWithVectorsInput, RagPreparedEnhancedSearchOutput, RagSearchResult,
};

pub(crate) use preparation::current_embedding_identity;
