use a4n_orchestrator::rag::EnhancedRagSearchResult;

use super::super::{RagEnhancedQuerySummary, RagEnhancedSearchOutput, RagSearchResult};

pub(super) fn project_enhanced_rag_search_output(
    result: EnhancedRagSearchResult,
) -> RagEnhancedSearchOutput {
    RagEnhancedSearchOutput {
        results: result
            .results
            .into_iter()
            .map(RagSearchResult::from)
            .collect(),
        queries: result
            .queries
            .iter()
            .map(RagEnhancedQuerySummary::from)
            .collect(),
        candidate_count: result.candidate_count,
    }
}
