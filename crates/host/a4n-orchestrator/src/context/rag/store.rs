use std::collections::HashMap;

use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_rag_core::vector_store::DeleteFilter;
use serde_json::Value;

use super::super::OrchestratorContext;
use a4n_rag_core::{EnhancedRagSearchResult, PreparedEnhancedRagSearch, RagEmbeddedInput};

use crate::rag::{RagEmbeddingInput, embed_rag_records, store_embedded_rag_records_in_store};

impl OrchestratorContext {
    pub(crate) async fn rag_embed_and_store_record(
        &self,
        project_id: i64,
        chunk_id: &str,
        text: &str,
        metadata: Value,
    ) -> Result<Vec<f32>, AppError> {
        let (embedded, identity) = embed_rag_records(
            vec![RagEmbeddingInput {
                chunk_id: chunk_id.to_string(),
                text: text.to_string(),
                metadata,
            }],
            self.embedding.as_ref(),
        )
        .await?;
        let vector = embedded
            .first()
            .map(|input| input.embedding.clone())
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("单条 RAG 向量生成后为空")))?;
        self.rag_store_embedded_records(project_id, embedded, identity)
            .await?;
        Ok(vector)
    }

    pub(crate) async fn rag_embed_and_store_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddingInput>,
    ) -> Result<usize, AppError> {
        if inputs.is_empty() {
            return Ok(0);
        }
        let (embedded, identity) = embed_rag_records(inputs, self.embedding.as_ref()).await?;
        self.rag_store_embedded_records(project_id, embedded, identity)
            .await
    }

    pub(crate) async fn rag_store_embedded_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddedInput>,
        embedding_identity: EmbeddingModelIdentity,
    ) -> Result<usize, AppError> {
        let vector_store = self.rag_store.as_ref();
        a4n_db::transaction::run_in_transaction(
            self.database.executor(),
            "写入 RAG 向量",
            |tx| {
                Box::pin(async move {
                    let result = async {
                        a4n_db::generation::require_project_owner_lifecycle_active(&tx, project_id)
                            .await
                            .map_err(crate::errors::generation_lifecycle_gate_error_to_app_error)?;
                        store_embedded_rag_records_in_store(
                            vector_store,
                            project_id,
                            inputs,
                            embedding_identity,
                        )
                        .await
                    }
                    .await;
                    (tx, result)
                }) as a4n_db::transaction::TransactionWork<'_, usize>
            },
        )
        .await
    }

    pub(crate) async fn rag_drop_project_collection(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        self.rag_store
            .drop_collection(&project_id.to_string())
            .await
            .map_err(AppError::from)
    }

    pub(crate) async fn rag_delete_project_records(
        &self,
        project_id: i64,
        filter: DeleteFilter,
    ) -> Result<usize, AppError> {
        self.rag_store
            .delete(&project_id.to_string(), filter)
            .await
            .map_err(AppError::from)
    }

    pub(crate) async fn rag_project_metadata_by_filter(
        &self,
        project_id: i64,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, Value>, AppError> {
        self.rag_store
            .metadata_by_filter(&project_id.to_string(), filter)
            .await
            .map_err(AppError::from)
    }

    pub(crate) async fn rag_project_metadata_by_ids(
        &self,
        project_id: i64,
        ids: &[String],
    ) -> Result<HashMap<String, Value>, AppError> {
        self.rag_store
            .metadata_by_ids(&project_id.to_string(), ids)
            .await
            .map_err(AppError::from)
    }

    pub(crate) async fn execute_prepared_chapter_generation_rag_search(
        &self,
        prepared: &crate::chapter_generation::PreparedGenerationRagSearch,
    ) -> Result<Option<String>, String> {
        crate::chapter_generation::rag_context::execute_prepared_chapter_generation_rag_search_in_store(
            self.rag_store.as_ref(),
            self.embedding.as_ref(),
            prepared,
        )
        .await
    }

    pub(crate) async fn execute_prepared_chapter_generation_rag_search_with_vectors(
        &self,
        prepared: &crate::chapter_generation::PreparedGenerationRagSearch,
        embedding_identity: &a4n_embedding_provider_contract::EmbeddingModelIdentity,
        query_vectors: Vec<Vec<f32>>,
    ) -> Result<Option<String>, String> {
        crate::chapter_generation::rag_context::execute_prepared_chapter_generation_rag_search_with_vectors_in_store(
            self.rag_store.as_ref(),
            prepared,
            embedding_identity,
            query_vectors,
        )
        .await
    }

    pub(crate) async fn execute_prepared_enhanced_rag_search(
        &self,
        prepared: PreparedEnhancedRagSearch,
    ) -> Result<EnhancedRagSearchResult, a4n_error::AppError> {
        crate::rag::search::enhanced::execute_prepared_enhanced_rag_search_in_store(
            self.rag_store.as_ref(),
            self.embedding.as_ref(),
            prepared,
        )
        .await
    }

    pub(crate) async fn execute_prepared_enhanced_rag_search_with_vectors(
        &self,
        prepared: PreparedEnhancedRagSearch,
        embedding_identity: &a4n_embedding_provider_contract::EmbeddingModelIdentity,
        query_vectors: Vec<Vec<f32>>,
    ) -> Result<EnhancedRagSearchResult, a4n_error::AppError> {
        crate::rag::search::enhanced::execute_prepared_enhanced_rag_search_with_vectors_in_store(
            self.rag_store.as_ref(),
            prepared,
            embedding_identity,
            query_vectors,
        )
        .await
    }
}
