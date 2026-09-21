use a4n_embedding_provider_contract::EmbeddingProvider;
use a4n_error::AppError;
use a4n_rag_core::{
    errors::{embedding_provider_error, vector_store_error},
    validation::validate_embedding_batch,
    vector_store::{SearchFilter, SearchResult, VectorStore},
};

pub(crate) async fn embed_queries_in_batch(
    provider: &dyn EmbeddingProvider,
    queries: &[String],
) -> Result<Vec<Vec<f32>>, AppError> {
    if queries.is_empty() {
        return Ok(Vec::new());
    }

    let (embeddings, identity) = provider
        .embed_with_identity(queries.to_vec())
        .await
        .map_err(embedding_provider_error)?;
    validate_embedding_batch(&embeddings, queries.len(), identity.dimension)?;
    Ok(embeddings)
}

pub(crate) async fn hybrid_rag_search_with_validated_embedding_in_store(
    vector_store: &dyn VectorStore,
    project_id: i64,
    query: &str,
    query_embedding: Vec<f32>,
    top_k: usize,
    filter: Option<SearchFilter>,
) -> Result<Vec<SearchResult>, AppError> {
    vector_store
        .hybrid_search(
            &project_id.to_string(),
            query,
            query_embedding,
            top_k,
            filter,
        )
        .await
        .map_err(vector_store_error)
        .map_err(AppError::from)
}
