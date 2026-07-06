use a4n_core::error::AppError;

use super::RagService;

impl RagService {
    pub(super) fn required_chunk_id(chunk_id: &str) -> Result<String, AppError> {
        let chunk_id = chunk_id.trim();
        if chunk_id.is_empty() {
            return Err(AppError::InvalidArgument(
                "RAG chunk_id 不能为空".to_string(),
            ));
        }
        Ok(chunk_id.to_string())
    }

    pub(super) fn required_content(text: &str) -> Result<String, AppError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(AppError::InvalidArgument(
                "RAG content 不能为空".to_string(),
            ));
        }
        Ok(text.to_string())
    }

    pub(super) fn validate_embedding_count(
        embeddings: &[Vec<f32>],
        expected: usize,
    ) -> Result<(), AppError> {
        if embeddings.len() != expected {
            return Err(AppError::EmbeddingError(format!(
                "嵌入返回数量不匹配：expected={expected}, actual={}",
                embeddings.len()
            )));
        }
        if embeddings.iter().any(Vec::is_empty) {
            return Err(AppError::EmbeddingError("嵌入返回空向量".to_string()));
        }
        Ok(())
    }

    pub(super) fn validate_embedding_batch(
        embeddings: &[Vec<f32>],
        expected_count: usize,
        expected_dimension: usize,
    ) -> Result<(), AppError> {
        Self::validate_embedding_count(embeddings, expected_count)?;
        for (index, embedding) in embeddings.iter().enumerate() {
            if embedding.len() != expected_dimension {
                return Err(AppError::EmbeddingError(format!(
                    "嵌入返回维度不匹配：index={index}, expected={expected_dimension}, actual={}",
                    embedding.len()
                )));
            }
            if embedding.iter().any(|value| !value.is_finite()) {
                return Err(AppError::EmbeddingError(format!(
                    "嵌入返回包含非有限数：index={index}"
                )));
            }
        }
        Ok(())
    }
}
