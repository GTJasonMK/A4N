use a4n_embedding_provider_contract::{EmbeddingModelIdentity, EmbeddingProvider};
use a4n_error::AppError;
use a4n_rag_core::{
    RagEmbeddedInput,
    errors::embedding_provider_error,
    validation::{required_chunk_id, required_content, validate_embedding_batch},
};

use super::RagEmbeddingInput;
#[cfg(test)]
use super::embedded::store_embedded_rag_records_in_store;
#[cfg(test)]
use a4n_rag_core::vector_store::VectorStore;

pub(crate) async fn embed_rag_records(
    inputs: Vec<RagEmbeddingInput>,
    provider: &dyn EmbeddingProvider,
) -> Result<(Vec<RagEmbeddedInput>, EmbeddingModelIdentity), AppError> {
    if inputs.is_empty() {
        return Err(AppError::InvalidArgument(
            "RAG embedding inputs cannot be empty".to_string(),
        ));
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
    let (embeddings, identity) = provider
        .embed_with_identity(texts)
        .await
        .map_err(embedding_provider_error)?;
    validate_embedding_batch(&embeddings, inputs.len(), identity.dimension)?;
    let embedded = inputs
        .into_iter()
        .zip(embeddings)
        .map(|((chunk_id, text, metadata), embedding)| RagEmbeddedInput {
            chunk_id,
            text,
            metadata,
            embedding,
        })
        .collect();
    Ok((embedded, identity))
}

#[cfg(test)]
pub(crate) async fn embed_and_store_rag_record_in_store(
    vector_store: &dyn VectorStore,
    project_id: i64,
    chunk_id: &str,
    text: &str,
    provider: &dyn EmbeddingProvider,
    metadata: serde_json::Value,
) -> Result<Vec<f32>, AppError> {
    let (embedded, identity) = embed_rag_records(
        vec![RagEmbeddingInput {
            chunk_id: chunk_id.to_string(),
            text: text.to_string(),
            metadata,
        }],
        provider,
    )
    .await?;
    let vector = embedded
        .first()
        .map(|input| input.embedding.clone())
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("单条 RAG 向量生成后为空")))?;
    store_embedded_rag_records_in_store(vector_store, project_id, embedded, identity).await?;
    Ok(vector)
}

#[cfg(test)]
pub(crate) async fn embed_and_store_rag_records_in_store(
    vector_store: &dyn VectorStore,
    project_id: i64,
    inputs: Vec<RagEmbeddingInput>,
    provider: &dyn EmbeddingProvider,
) -> Result<usize, AppError> {
    if inputs.is_empty() {
        return Ok(0);
    }
    let (embedded, identity) = embed_rag_records(inputs, provider).await?;
    store_embedded_rag_records_in_store(vector_store, project_id, embedded, identity).await
}
