use a4n_embedding::provider::{EmbeddingModelIdentity, EmbeddingProvider};
use a4n_rag_core::vector_store::VectorStore;

use super::{
    contracts::{GenerationRagContext, PreparedGenerationRagSearch},
    fusion::fuse_generation_rag_results,
    projection::generation_rag_context_from_results,
    rules::{validate_generation_embedding_identity_match, validate_generation_query_vectors},
};
use crate::rag::{hybrid_rag_search_filtered_in_store, hybrid_rag_search_with_embedding_in_store};

const GENERATION_RAG_PULL_K: usize = 24;

pub(crate) async fn execute_prepared_chapter_generation_rag_search_in_store(
    store: &dyn VectorStore,
    provider: &dyn EmbeddingProvider,
    prepared: &PreparedGenerationRagSearch,
) -> Result<GenerationRagContext, String> {
    let weighted_queries = prepared.queries.weighted_queries();
    if weighted_queries.is_empty() {
        return Ok(GenerationRagContext::default());
    }
    let mut batches = Vec::new();
    for query in weighted_queries {
        let results = hybrid_rag_search_filtered_in_store(
            store,
            prepared.project_id,
            query.text,
            provider,
            GENERATION_RAG_PULL_K,
            Some(prepared.filter.clone()),
        )
        .await
        .map_err(|error| error.to_string())?;
        batches.push((query.weight, results));
    }
    Ok(build_generation_rag_context(
        prepared,
        prepared.project_id,
        prepared.chapter_number,
        batches,
    ))
}

pub(crate) async fn execute_prepared_chapter_generation_rag_search_with_vectors_in_store(
    store: &dyn VectorStore,
    prepared: &PreparedGenerationRagSearch,
    embedding_identity: &EmbeddingModelIdentity,
    query_vectors: Vec<Vec<f32>>,
) -> Result<GenerationRagContext, String> {
    validate_generation_embedding_identity_match(embedding_identity, &prepared.embedding_identity)?;
    let weighted_queries = prepared.queries.weighted_queries();
    validate_generation_query_vectors(
        &query_vectors,
        weighted_queries.len(),
        prepared.embedding_identity.dimension,
    )?;
    if weighted_queries.is_empty() {
        return Ok(GenerationRagContext::default());
    }
    let mut batches = Vec::new();
    for (query, vector) in weighted_queries.into_iter().zip(query_vectors) {
        let results = hybrid_rag_search_with_embedding_in_store(
            store,
            prepared.project_id,
            query.text,
            vector,
            &prepared.embedding_identity,
            GENERATION_RAG_PULL_K,
            Some(prepared.filter.clone()),
        )
        .await
        .map_err(|error| error.to_string())?;
        batches.push((query.weight, results));
    }
    Ok(build_generation_rag_context(
        prepared,
        prepared.project_id,
        prepared.chapter_number,
        batches,
    ))
}

fn build_generation_rag_context(
    prepared: &PreparedGenerationRagSearch,
    project_id: i64,
    chapter_number: i32,
    batches: Vec<(f32, Vec<a4n_rag_core::vector_store::SearchResult>)>,
) -> GenerationRagContext {
    generation_rag_context_from_results(
        fuse_generation_rag_results(batches, chapter_number),
        &prepared.queries,
        project_id,
        chapter_number,
    )
}
