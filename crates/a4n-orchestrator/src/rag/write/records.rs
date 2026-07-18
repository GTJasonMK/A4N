use a4n_core::error::AppError;
use a4n_rag_core::{
    errors::vector_store_error,
    validation::ensure_embedding_write_count,
    vector_store::{EmbeddingRecord, VectorStore},
};

pub(super) async fn store_rag_embedding_records(
    vector_store: &dyn VectorStore,
    project_id: i64,
    records: Vec<EmbeddingRecord>,
) -> Result<usize, AppError> {
    let expected = records.len();
    let written = vector_store
        .add_embeddings(&project_id.to_string(), records)
        .await
        .map_err(vector_store_error)?;
    ensure_embedding_write_count(expected, written)?;
    Ok(written)
}
