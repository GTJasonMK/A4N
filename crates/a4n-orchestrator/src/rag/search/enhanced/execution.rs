use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingModelIdentity, EmbeddingProvider};
use a4n_rag_core::vector_store::{SearchResult, VectorStore};

use super::{
    contracts::{EnhancedRagSearchResult, PreparedEnhancedRagSearch},
    fusion::fuse_enhanced_rag_results,
    rules::{validate_embedding_identity_match, validate_query_vectors},
};
use crate::rag::search::execution::{
    hybrid_rag_search_filtered_in_store, hybrid_rag_search_with_embedding_in_store,
};

pub(crate) async fn execute_prepared_enhanced_rag_search_in_store(
    store: &dyn VectorStore,
    provider: &dyn EmbeddingProvider,
    project_id: i64,
    prepared: &PreparedEnhancedRagSearch,
) -> Result<EnhancedRagSearchResult, AppError> {
    if prepared.queries.is_empty() {
        return Ok(empty_enhanced_rag_search_result());
    }
    let mut batches = Vec::new();
    for query in &prepared.queries {
        let results = hybrid_rag_search_filtered_in_store(
            store,
            project_id,
            &query.query,
            provider,
            prepared.pull_k,
            Some(prepared.filter.clone()),
        )
        .await?;
        batches.push((query.weight, results));
    }
    build_enhanced_rag_search_result(prepared, batches)
}

pub(crate) async fn execute_prepared_enhanced_rag_search_with_vectors_in_store(
    store: &dyn VectorStore,
    project_id: i64,
    prepared: &PreparedEnhancedRagSearch,
    embedding_identity: &EmbeddingModelIdentity,
    query_vectors: Vec<Vec<f32>>,
) -> Result<EnhancedRagSearchResult, AppError> {
    validate_embedding_identity_match(embedding_identity, &prepared.embedding_identity)?;
    validate_query_vectors(
        &query_vectors,
        prepared.queries.len(),
        embedding_identity.dimension,
    )?;
    if prepared.queries.is_empty() {
        return Ok(empty_enhanced_rag_search_result());
    }
    let mut batches = Vec::new();
    for (query, vector) in prepared.queries.iter().zip(query_vectors) {
        let results = hybrid_rag_search_with_embedding_in_store(
            store,
            project_id,
            &query.query,
            vector,
            &prepared.embedding_identity,
            prepared.pull_k,
            Some(prepared.filter.clone()),
        )
        .await?;
        batches.push((query.weight, results));
    }
    build_enhanced_rag_search_result(prepared, batches)
}

fn build_enhanced_rag_search_result(
    prepared: &PreparedEnhancedRagSearch,
    batches: Vec<(f32, Vec<SearchResult>)>,
) -> Result<EnhancedRagSearchResult, AppError> {
    let candidate_count = batches.iter().map(|(_, results)| results.len()).sum();
    let results = fuse_enhanced_rag_results(batches, prepared.chapter_number)?
        .into_iter()
        .take(prepared.top_k)
        .collect();
    Ok(EnhancedRagSearchResult {
        results,
        queries: prepared.queries.clone(),
        candidate_count,
    })
}

fn empty_enhanced_rag_search_result() -> EnhancedRagSearchResult {
    EnhancedRagSearchResult {
        results: Vec::new(),
        queries: Vec::new(),
        candidate_count: 0,
    }
}
