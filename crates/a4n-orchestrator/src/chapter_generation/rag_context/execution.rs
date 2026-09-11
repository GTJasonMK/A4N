use a4n_embedding::provider::{EmbeddingModelIdentity, EmbeddingProvider};
use a4n_rag_core::{
    validation::{validate_embedding_identity_match, validate_query_vectors},
    vector_store::VectorStore,
};

use super::{
    contracts::PreparedGenerationRagSearch, formatting::format_generation_rag_context,
    fusion::fuse_generation_rag_results,
};
use crate::rag::{embed_queries_in_batch, hybrid_rag_search_with_validated_embedding_in_store};

const GENERATION_RAG_PULL_K: usize = 24;

pub(crate) async fn execute_prepared_chapter_generation_rag_search_in_store(
    store: &dyn VectorStore,
    provider: &dyn EmbeddingProvider,
    prepared: &PreparedGenerationRagSearch,
) -> Result<Option<String>, String> {
    let query_texts = prepared
        .queries
        .iter()
        .map(|query| query.text.clone())
        .collect::<Vec<_>>();
    let query_vectors = embed_queries_in_batch(provider, &query_texts)
        .await
        .map_err(|error| error.to_string())?;
    let mut batches = Vec::with_capacity(prepared.queries.len());
    for (query, vector) in prepared.queries.iter().zip(query_vectors) {
        let results = hybrid_rag_search_with_validated_embedding_in_store(
            store,
            prepared.project_id,
            &query.text,
            vector,
            GENERATION_RAG_PULL_K,
            Some(prepared.filter.clone()),
        )
        .await
        .map_err(|error| error.to_string())?;
        batches.push((query.weight, results));
    }
    Ok(build_generation_rag_context(prepared, batches))
}

pub(crate) async fn execute_prepared_chapter_generation_rag_search_with_vectors_in_store(
    store: &dyn VectorStore,
    prepared: &PreparedGenerationRagSearch,
    embedding_identity: &EmbeddingModelIdentity,
    query_vectors: Vec<Vec<f32>>,
) -> Result<Option<String>, String> {
    validate_embedding_identity_match(embedding_identity, &prepared.embedding_identity)
        .map_err(|error| error.to_string())?;
    validate_query_vectors(
        &query_vectors,
        prepared.queries.len(),
        prepared.embedding_identity.dimension,
    )
    .map_err(|error| error.to_string())?;
    let mut batches = Vec::new();
    for (query, vector) in prepared.queries.iter().zip(query_vectors) {
        let results = hybrid_rag_search_with_validated_embedding_in_store(
            store,
            prepared.project_id,
            &query.text,
            vector,
            GENERATION_RAG_PULL_K,
            Some(prepared.filter.clone()),
        )
        .await
        .map_err(|error| error.to_string())?;
        batches.push((query.weight, results));
    }
    Ok(build_generation_rag_context(prepared, batches))
}

fn build_generation_rag_context(
    prepared: &PreparedGenerationRagSearch,
    batches: Vec<(f32, Vec<a4n_rag_core::vector_store::SearchResult>)>,
) -> Option<String> {
    let results = fuse_generation_rag_results(batches, prepared.chapter_number);
    format_generation_rag_context(&results, prepared.chapter_number)
}
