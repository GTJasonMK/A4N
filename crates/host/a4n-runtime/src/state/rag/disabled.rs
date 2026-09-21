use crate::state::profile::RuntimeProfile;
use a4n_domain::ProviderError;
use a4n_rag_core::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use async_trait::async_trait;
use std::collections::HashMap;

pub(in crate::state) struct DisabledVectorStore {
    runtime_profile: RuntimeProfile,
}

impl DisabledVectorStore {
    pub(in crate::state) fn new(runtime_profile: RuntimeProfile) -> Self {
        Self { runtime_profile }
    }

    fn disabled_error(&self) -> ProviderError {
        ProviderError::Configuration(format!(
            "RAG 能力未启用：当前 runtime profile={}",
            self.runtime_profile.as_str()
        ))
    }
}

#[async_trait]
impl VectorStore for DisabledVectorStore {
    async fn add_embeddings(
        &self,
        _collection: &str,
        _records: Vec<EmbeddingRecord>,
    ) -> Result<usize, ProviderError> {
        Err(self.disabled_error())
    }

    async fn query(
        &self,
        _collection: &str,
        _query_embedding: Vec<f32>,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, ProviderError> {
        Err(self.disabled_error())
    }

    async fn search_text(
        &self,
        _collection: &str,
        _query_text: &str,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, ProviderError> {
        Err(self.disabled_error())
    }

    async fn hybrid_search(
        &self,
        _collection: &str,
        _query_text: &str,
        _query_embedding: Vec<f32>,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, ProviderError> {
        Err(self.disabled_error())
    }

    async fn delete(
        &self,
        _collection: &str,
        _filter: DeleteFilter,
    ) -> Result<usize, ProviderError> {
        Err(self.disabled_error())
    }

    async fn drop_collection(&self, _collection: &str) -> Result<(), ProviderError> {
        // 未启用 RAG 时不存在可删除的 collection；删除语义保持幂等成功。
        Ok(())
    }

    async fn metadata_by_ids(
        &self,
        _collection: &str,
        _ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, ProviderError> {
        Err(self.disabled_error())
    }

    async fn metadata_by_filter(
        &self,
        _collection: &str,
        _filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, ProviderError> {
        Err(self.disabled_error())
    }

    async fn completeness_report_by_ids(
        &self,
        _collection: &str,
        _expected_ids: &[String],
    ) -> Result<CompletenessReport, ProviderError> {
        Err(self.disabled_error())
    }
}
