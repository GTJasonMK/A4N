use a4n_core::error::AppError;
use a4n_engine::embedding::provider::{EmbeddingModelIdentity, EmbeddingProvider};
use a4n_engine::rag::vector_store::EmbeddingRecord;

use super::{RagEmbeddingInput, RagService};

impl RagService {
    /// 对文本进行向量化并存储
    pub async fn embed_and_store(
        &self,
        project_id: i64,
        chunk_id: &str,
        text: &str,
        provider: &dyn EmbeddingProvider,
        metadata: serde_json::Value,
    ) -> Result<Vec<f32>, AppError> {
        let chunk_id = Self::required_chunk_id(chunk_id)?;
        let text = Self::required_content(text)?;
        let (embeddings, embedding_identity) = provider
            .embed_with_identity(vec![text.clone()])
            .await
            .map_err(|e| AppError::EmbeddingError(e.to_string()))?;
        Self::validate_embedding_batch(&embeddings, 1, embedding_identity.dimension)?;

        let vector = embeddings
            .into_iter()
            .next()
            .expect("validated one embedding");

        let record = EmbeddingRecord {
            id: chunk_id,
            embedding: vector.clone(),
            metadata: Self::normalize_metadata(project_id, &text, metadata, &embedding_identity)?,
            content: Some(text),
        };

        self.vector_store
            .add_embeddings(&project_id.to_string(), vec![record])
            .await
            .map_err(|e| AppError::VectorStoreError(e.to_string()))?;

        Ok(vector)
    }

    /// 批量向量化并存储，保留每条记录自己的 metadata。
    pub async fn embed_and_store_records(
        &self,
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
                    Self::required_chunk_id(&input.chunk_id)?,
                    Self::required_content(&input.text)?,
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
            .map_err(|e| AppError::EmbeddingError(e.to_string()))?;
        Self::validate_embedding_batch(&embeddings, inputs.len(), embedding_identity.dimension)?;

        let records = inputs
            .into_iter()
            .zip(embeddings)
            .map(|((id, text, metadata), embedding)| {
                Ok(EmbeddingRecord {
                    id,
                    embedding,
                    metadata: Self::normalize_metadata(
                        project_id,
                        &text,
                        metadata,
                        &embedding_identity,
                    )?,
                    content: Some(text),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        let expected = records.len();
        let written = self
            .vector_store
            .add_embeddings(&project_id.to_string(), records)
            .await
            .map_err(|e| AppError::VectorStoreError(e.to_string()))?;
        if written != expected {
            return Err(AppError::VectorStoreError(format!(
                "向量存储写入数量不匹配：expected={expected}, actual={written}"
            )));
        }
        Ok(written)
    }

    /// 存储调用方已计算好的嵌入向量。
    ///
    /// 该路径用于远程后端 + 本地桌面端：服务器仍负责 canonical metadata，
    /// 客户端只提交文本对应的向量。
    pub async fn store_embedded_records(
        &self,
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
                    Self::required_chunk_id(&input.chunk_id)?,
                    Self::required_content(&input.text)?,
                    input.metadata,
                    input.embedding,
                ))
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let embeddings = inputs
            .iter()
            .map(|(_, _, _, embedding)| embedding.clone())
            .collect::<Vec<_>>();
        Self::validate_embedding_batch(&embeddings, inputs.len(), embedding_identity.dimension)?;

        let records = inputs
            .into_iter()
            .map(|(id, text, metadata, embedding)| {
                Ok(EmbeddingRecord {
                    id,
                    embedding,
                    metadata: Self::normalize_metadata(
                        project_id,
                        &text,
                        metadata,
                        &embedding_identity,
                    )?,
                    content: Some(text),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        let expected = records.len();
        let written = self
            .vector_store
            .add_embeddings(&project_id.to_string(), records)
            .await
            .map_err(|e| AppError::VectorStoreError(e.to_string()))?;
        if written != expected {
            return Err(AppError::VectorStoreError(format!(
                "向量存储写入数量不匹配：expected={expected}, actual={written}"
            )));
        }
        Ok(written)
    }
}

#[derive(Debug, Clone)]
pub struct RagEmbeddedInput {
    pub chunk_id: String,
    pub text: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
}
