use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingModelIdentity, EmbeddingProvider};
use a4n_rag_core::vector_store::{SearchResult, VectorStore};

use super::{
    contracts::{EnhancedRagSearchResult, PreparedEnhancedRagSearch},
    fusion::fuse_enhanced_rag_results,
    rules::{enhanced_rag_pull_k, validate_embedding_identity_match, validate_query_vectors},
};
use crate::rag::search::execution::{
    embed_queries_in_batch, hybrid_rag_search_with_validated_embedding_in_store,
};

pub(crate) async fn execute_prepared_enhanced_rag_search_in_store(
    store: &dyn VectorStore,
    provider: &dyn EmbeddingProvider,
    prepared: PreparedEnhancedRagSearch,
) -> Result<EnhancedRagSearchResult, AppError> {
    let pull_k = enhanced_rag_pull_k(prepared.top_k);
    let query_texts = prepared
        .queries
        .iter()
        .map(|query| query.query.clone())
        .collect::<Vec<_>>();
    let query_vectors = embed_queries_in_batch(provider, &query_texts).await?;
    let mut batches = Vec::with_capacity(prepared.queries.len());
    for (query, vector) in prepared.queries.iter().zip(query_vectors) {
        let results = hybrid_rag_search_with_validated_embedding_in_store(
            store,
            prepared.project_id,
            &query.query,
            vector,
            pull_k,
            Some(prepared.filter.clone()),
        )
        .await?;
        batches.push((query.weight, results));
    }
    Ok(build_enhanced_rag_search_result(prepared, batches))
}

pub(crate) async fn execute_prepared_enhanced_rag_search_with_vectors_in_store(
    store: &dyn VectorStore,
    prepared: PreparedEnhancedRagSearch,
    embedding_identity: &EmbeddingModelIdentity,
    query_vectors: Vec<Vec<f32>>,
) -> Result<EnhancedRagSearchResult, AppError> {
    validate_embedding_identity_match(embedding_identity, &prepared.embedding_identity)?;
    validate_query_vectors(
        &query_vectors,
        prepared.queries.len(),
        prepared.embedding_identity.dimension,
    )?;
    let pull_k = enhanced_rag_pull_k(prepared.top_k);
    let mut batches = Vec::new();
    for (query, vector) in prepared.queries.iter().zip(query_vectors) {
        let results = hybrid_rag_search_with_validated_embedding_in_store(
            store,
            prepared.project_id,
            &query.query,
            vector,
            pull_k,
            Some(prepared.filter.clone()),
        )
        .await?;
        batches.push((query.weight, results));
    }
    Ok(build_enhanced_rag_search_result(prepared, batches))
}

fn build_enhanced_rag_search_result(
    prepared: PreparedEnhancedRagSearch,
    batches: Vec<(f32, Vec<SearchResult>)>,
) -> EnhancedRagSearchResult {
    let candidate_count = batches.iter().map(|(_, results)| results.len()).sum();
    let results = fuse_enhanced_rag_results(batches, prepared.chapter_number)
        .into_iter()
        .take(prepared.top_k)
        .collect();
    EnhancedRagSearchResult {
        results,
        queries: prepared.queries,
        candidate_count,
    }
}
