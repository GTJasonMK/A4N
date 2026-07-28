use a4n_orchestrator::rag::EnhancedRagSearchResult;

use super::super::{RagEnhancedSearchOutput, rag_enhanced_query_summary, rag_search_result};

pub(super) fn project_enhanced_rag_search_output(
    result: EnhancedRagSearchResult,
) -> RagEnhancedSearchOutput {
    RagEnhancedSearchOutput {
        results: result.results.into_iter().map(rag_search_result).collect(),
        queries: result
            .queries
            .iter()
            .map(rag_enhanced_query_summary)
            .collect(),
        candidate_count: result.candidate_count,
    }
}
