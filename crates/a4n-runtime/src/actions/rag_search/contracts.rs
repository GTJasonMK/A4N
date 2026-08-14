pub use a4n_api_contract::rag::{
    EnhancedSearchInput, RagEnhancedQuerySummary, RagEnhancedSearchOutput,
    RagEnhancedSearchWithVectorsInput, RagPreparedEnhancedSearchOutput, RagSearchResult,
};
use a4n_orchestrator::rag::EnhancedRagQuery;
use a4n_rag_core::vector_store::SearchResult;

pub(super) fn rag_search_result(result: SearchResult) -> RagSearchResult {
    RagSearchResult {
        id: result.id,
        content: result.content,
        score: result.score,
        vector_score: result.vector_score,
        bm25_score: result.bm25_score,
        metadata: result.metadata,
    }
}

pub(super) fn rag_enhanced_query_summary(query: &EnhancedRagQuery) -> RagEnhancedQuerySummary {
    RagEnhancedQuerySummary {
        kind: query.kind.to_string(),
        query: query.query.clone(),
        weight: query.weight,
    }
}
