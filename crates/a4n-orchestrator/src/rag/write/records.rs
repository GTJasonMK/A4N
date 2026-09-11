use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_rag_core::{
    errors::vector_store_error,
    metadata::normalize_metadata,
    validation::ensure_embedding_write_count,
    vector_store::{EmbeddingRecord, VectorStore},
};

pub(super) fn embedding_record(
    id: String,
    text: String,
    metadata: serde_json::Value,
    embedding: Vec<f32>,
    embedding_identity: &EmbeddingModelIdentity,
) -> Result<EmbeddingRecord, AppError> {
    let metadata = normalize_metadata(&text, metadata, embedding_identity)?;
    Ok(EmbeddingRecord {
        id,
        embedding,
        metadata,
        content: text,
    })
}

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
