use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::{
    validation::{required_chunk_id, required_content, validate_embedding_batch},
    vector_store::VectorStore,
};

use super::contracts::RagEmbeddedInput;
use super::records::{embedding_record, store_rag_embedding_records};

pub(crate) async fn store_embedded_rag_records_in_store(
    vector_store: &dyn VectorStore,
    project_id: i64,
    inputs: Vec<RagEmbeddedInput>,
    embedding_identity: EmbeddingModelIdentity,
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
                input.embedding,
            ))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let embeddings = inputs
        .iter()
        .map(|(_, _, _, embedding)| embedding.clone())
        .collect::<Vec<_>>();
    validate_embedding_batch(&embeddings, inputs.len(), embedding_identity.dimension)?;
    let records = inputs
        .into_iter()
        .map(|(id, text, metadata, embedding)| {
            embedding_record(id, text, metadata, embedding, &embedding_identity)
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    store_rag_embedding_records(vector_store, project_id, records).await
}
