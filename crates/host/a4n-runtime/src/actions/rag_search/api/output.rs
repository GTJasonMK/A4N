use a4n_rag_core::EnhancedRagSearchResult;

use super::super::{RagEnhancedQuerySummary, RagEnhancedSearchOutput, RagSearchResult};

pub(super) fn project_enhanced_rag_search_output(
    result: EnhancedRagSearchResult,
) -> RagEnhancedSearchOutput {
    RagEnhancedSearchOutput {
        results: result
            .results
            .into_iter()
            .map(|result| RagSearchResult {
                id: result.id,
                content: result.content,
                score: result.score,
                metadata: result.metadata,
            })
            .collect(),
        queries: result
            .queries
            .into_iter()
            .map(|query| RagEnhancedQuerySummary {
                label: query.label.to_string(),
                query: query.query,
            })
            .collect(),
        candidate_count: result.candidate_count,
    }
}
