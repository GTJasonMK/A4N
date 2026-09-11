//! Narrow application boundary for enhanced RAG search.
//!
//! The contract carries prepared search and ingest values only. Database
//! queries and vector-store ownership stay in the implementing application.

use std::collections::HashMap;

use a4n_domain::rag::RagDataType;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_rag_core::vector_store::SearchFilter;
use a4n_rag_core::{
    EnhancedRagQuery, EnhancedRagSearchResult, PreparedEnhancedRagSearch, PreparedRagRecordSync,
    RagEmbeddedInput,
};
use async_trait::async_trait;
use serde_json::Value;

/// Snapshot used by Runtime's pure record comparison loop.  The implementing
/// application owns how metadata and stale IDs are read; Runtime only decides
/// which records need embedding and submits the resulting batches.
#[derive(Debug, Clone)]
pub struct RagIngestPreparation {
    pub embedding_identity: EmbeddingModelIdentity,
    pub stored_metadata: HashMap<String, Value>,
    pub stale_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagRecordSet {
    All,
    Outline,
    BlueprintMaterial,
    Protagonist,
    Scene,
}

#[async_trait]
pub trait RagSearchPort: Send + Sync {
    async fn embedding_model_identity(&self) -> Result<EmbeddingModelIdentity, AppError>;

    async fn prepare_enhanced_rag_queries(
        &self,
        project_id: i64,
        base_query: &str,
        chapter_number: Option<i32>,
    ) -> Result<Vec<EnhancedRagQuery>, AppError>;

    async fn enhanced_rag_search_filter(
        &self,
        project_id: i64,
        embedding_identity: &EmbeddingModelIdentity,
        data_types: Option<Vec<String>>,
    ) -> Result<SearchFilter, AppError>;

    async fn execute_prepared_enhanced_rag_search(
        &self,
        prepared: PreparedEnhancedRagSearch,
    ) -> Result<EnhancedRagSearchResult, AppError>;

    async fn execute_prepared_enhanced_rag_search_with_vectors(
        &self,
        prepared: PreparedEnhancedRagSearch,
        embedding_identity: &EmbeddingModelIdentity,
        query_vectors: Vec<Vec<f32>>,
    ) -> Result<EnhancedRagSearchResult, AppError>;
}

/// Narrow application boundary for RAG ingestion and synchronization.
#[async_trait]
pub trait RagIngestPort: Send + Sync {
    async fn sync_chapter_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_mode: a4n_rag_core::ChapterRagChunkingMode,
    ) -> Result<u32, AppError>;

    async fn expected_records(
        &self,
        project_id: i64,
        set: RagRecordSet,
        chapter_mode: a4n_rag_core::ChapterRagChunkingMode,
    ) -> Result<Vec<a4n_rag_core::RagRecord>, AppError>;

    async fn prepare_rag_sync(
        &self,
        project_id: i64,
        data_types: Option<Vec<RagDataType>>,
    ) -> Result<RagIngestPreparation, AppError>;

    async fn project_rag_metadata(
        &self,
        project_id: i64,
    ) -> Result<HashMap<String, Value>, AppError>;

    async fn commit_embedded_rag_records(
        &self,
        project_id: i64,
        rebuild: bool,
        inputs: Vec<RagEmbeddedInput>,
        embedding_identity: EmbeddingModelIdentity,
        stale_ids: Vec<String>,
    ) -> Result<u32, AppError>;

    async fn clear_project_rag(&self, project_id: i64) -> Result<(), AppError>;

    async fn store_prepared_rag_records(
        &self,
        project_id: i64,
        records: Vec<PreparedRagRecordSync>,
    ) -> Result<(), AppError>;

    async fn cleanup_stale_rag_records(
        &self,
        project_id: i64,
        stale_ids: Vec<String>,
    ) -> Result<u32, AppError>;

    fn embedding_batch_size(&self) -> usize;
}
