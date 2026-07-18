use a4n_core::error::AppError;

use super::scoring;

#[derive(Debug, Clone)]
pub(super) struct StoredRecord {
    pub(super) id: String,
    pub(super) embedding: Vec<f32>,
    pub(super) content: String,
    pub(super) metadata: serde_json::Value,
}

pub(super) fn vector_store_error(message: impl Into<String>) -> AppError {
    AppError::VectorStoreError(message.into())
}

pub(super) fn validate_embedding_vector(
    collection: &str,
    id: &str,
    embedding: &[f32],
) -> Result<usize, AppError> {
    if embedding.is_empty() {
        return Err(vector_store_error(format!(
            "嵌入向量不能为空：collection={collection}, id={id}"
        )));
    }
    if embedding.iter().any(|value| !value.is_finite()) {
        return Err(vector_store_error(format!(
            "嵌入向量包含非有限数：collection={collection}, id={id}"
        )));
    }
    Ok(embedding.len())
}

pub(super) fn checked_cosine_sim(
    collection: &str,
    id: &str,
    query_embedding: &[f32],
    record_embedding: &[f32],
) -> Result<f32, AppError> {
    let query_dimension = validate_embedding_vector(collection, "<query>", query_embedding)?;
    let record_dimension = validate_embedding_vector(collection, id, record_embedding)?;
    if query_dimension != record_dimension {
        return Err(vector_store_error(format!(
            "嵌入向量维度不一致：collection={collection}, id={id}, query_dimension={query_dimension}, record_dimension={record_dimension}"
        )));
    }
    Ok(scoring::cosine_sim(query_embedding, record_embedding))
}
