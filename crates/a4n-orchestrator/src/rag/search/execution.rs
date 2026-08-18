use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingProvider;
use a4n_rag_core::{
    errors::{embedding_provider_error, vector_store_error},
    validation::validate_embedding_batch,
    vector_store::{SearchFilter, SearchResult, VectorStore},
};

pub(crate) async fn hybrid_rag_search_filtered_in_store(
    vector_store: &dyn VectorStore,
    project_id: i64,
    query: &str,
    provider: &dyn EmbeddingProvider,
    top_k: usize,
    filter: Option<SearchFilter>,
) -> Result<Vec<SearchResult>, AppError> {
    let (query_embedding, embedding_identity) = provider
        .embed_with_identity(vec![query.to_string()])
        .await
        .map_err(embedding_provider_error)?;
    validate_embedding_batch(&query_embedding, 1, embedding_identity.dimension)?;
    let query_embedding = query_embedding
        .into_iter()
        .next()
        .expect("validated one query embedding");
    hybrid_rag_search_with_validated_embedding_in_store(
        vector_store,
        project_id,
        query,
        query_embedding,
        top_k,
        filter,
    )
    .await
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
}
