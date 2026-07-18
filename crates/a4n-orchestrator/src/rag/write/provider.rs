use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingProvider;
use a4n_rag_core::{
    errors::embedding_provider_error,
    metadata::normalize_metadata,
    validation::{required_chunk_id, required_content, validate_embedding_batch},
    vector_store::{EmbeddingRecord, VectorStore},
};

use super::contracts::RagEmbeddingInput;
use super::records::store_rag_embedding_records;

pub(super) async fn embed_and_store_rag_record_in_store(
    vector_store: &dyn VectorStore,
    project_id: i64,
    chunk_id: &str,
    text: &str,
    provider: &dyn EmbeddingProvider,
    metadata: serde_json::Value,
) -> Result<Vec<f32>, AppError> {
    let chunk_id = required_chunk_id(chunk_id)?;
    let text = required_content(text)?;
    let (embeddings, embedding_identity) = provider
        .embed_with_identity(vec![text.clone()])
        .await
        .map_err(embedding_provider_error)?;
    validate_embedding_batch(&embeddings, 1, embedding_identity.dimension)?;
    let vector = embeddings
        .into_iter()
        .next()
        .expect("validated one embedding");
    let record = EmbeddingRecord {
        id: chunk_id,
        embedding: vector.clone(),
        metadata: normalize_metadata(project_id, &text, metadata, &embedding_identity)?,
        content: Some(text),
    };
    store_rag_embedding_records(vector_store, project_id, vec![record]).await?;
    Ok(vector)
}

pub(super) async fn embed_and_store_rag_records_in_store(
    vector_store: &dyn VectorStore,
    project_id: i64,
    inputs: Vec<RagEmbeddingInput>,
    provider: &dyn EmbeddingProvider,
) -> Result<usize, AppError> {
    if inputs.is_empty() {
        return Ok(0);
    }
    let inputs = inputs
        .into_iter()
        .map(|input| {
            Ok((
                required_chunk_id(&input.chunk_id)?,
                required_content(&input.text)?,
                input.metadata,
            ))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let texts = inputs
        .iter()
        .map(|(_, text, _)| text.clone())
        .collect::<Vec<_>>();
    let (embeddings, embedding_identity) = provider
        .embed_with_identity(texts.clone())
        .await
        .map_err(embedding_provider_error)?;
    validate_embedding_batch(&embeddings, inputs.len(), embedding_identity.dimension)?;
    let records = inputs
        .into_iter()
        .zip(embeddings)
        .map(|((id, text, metadata), embedding)| {
            Ok(EmbeddingRecord {
                id,
                embedding,
                metadata: normalize_metadata(project_id, &text, metadata, &embedding_identity)?,
                content: Some(text),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    store_rag_embedding_records(vector_store, project_id, records).await
}
