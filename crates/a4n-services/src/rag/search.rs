use a4n_core::error::AppError;
use a4n_engine::embedding::provider::EmbeddingModelIdentity;
use a4n_engine::embedding::provider::EmbeddingProvider;
use a4n_engine::rag::vector_store::{SearchFilter, SearchResult};

use super::RagService;

impl RagService {
    /// 带过滤条件的混合搜索，用于生成流程只检索特定类型或章节范围的上下文。
    pub async fn hybrid_search_filtered(
        &self,
        project_id: i64,
        query: &str,
        provider: &dyn EmbeddingProvider,
        top_k: usize,
        filter: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let (query_embedding, embedding_identity) = provider
            .embed_with_identity(vec![query.to_string()])
            .await
            .map_err(|e| AppError::EmbeddingError(e.to_string()))?;
        Self::validate_embedding_batch(&query_embedding, 1, embedding_identity.dimension)?;

        let query_vec = query_embedding
            .into_iter()
            .next()
            .expect("validated one query embedding");

        let results = self
            .vector_store
            .hybrid_search(
                &project_id.to_string(),
                query,
                query_vec,
                top_k,
                filter,
                None,
            )
            .await
            .map_err(|e| AppError::VectorStoreError(e.to_string()))?;

        Ok(results)
    }

    /// 使用调用方已计算好的 query embedding 执行混合搜索。
    pub async fn hybrid_search_with_embedding(
        &self,
        project_id: i64,
        query: &str,
        query_embedding: Vec<f32>,
        embedding_identity: &EmbeddingModelIdentity,
        top_k: usize,
        filter: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        Self::validate_embedding_batch(
            std::slice::from_ref(&query_embedding),
            1,
            embedding_identity.dimension,
        )?;

        let results = self
            .vector_store
            .hybrid_search(
                &project_id.to_string(),
                query,
                query_embedding,
                top_k,
                filter,
                None,
            )
            .await
            .map_err(|e| AppError::VectorStoreError(e.to_string()))?;

        Ok(results)
    }
}
