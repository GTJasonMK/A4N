//! HybridVectorStore：把任意 VectorStore 与 Tantivy BM25 索引组合为带原生全文搜索的存储
//!
//! 设计：
//! - `inner` 负责向量端（add/query/delete/集合管理）
//! - `text` 负责 BM25 端（同步 upsert content、原生 BM25 搜索）
//! - `hybrid_search` 双端调用并通过 RRF 融合
//!
//! 写入时通过 `add_embeddings` 同时维护两端；
//! 删除时 `delete` 同步 inner，并用稳定 id 前缀同步清理 Tantivy；
//! `drop_collection` 双端同步清空。

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use a4n_core::error::AppError;
use async_trait::async_trait;

use super::fusion::{RRF_DEFAULT_K, rrf_fuse};
use super::tantivy_index::TantivyTextIndex;
use super::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use filters::{filter_bm25_results_by_metadata, requires_precise_bm25_filter};
use text_records::extract_text_records;

mod filters;
mod text_records;

#[cfg(test)]
mod tests;

/// 混合存储：向量后端 + Tantivy BM25
pub struct HybridVectorStore {
    inner: Arc<dyn VectorStore>,
    text: Arc<TantivyTextIndex>,
}

impl HybridVectorStore {
    pub fn new(inner: Arc<dyn VectorStore>, text: Arc<TantivyTextIndex>) -> Self {
        Self { inner, text }
    }
}

#[async_trait]
impl VectorStore for HybridVectorStore {
    async fn add_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        let text_records = extract_text_records(&records);
        let n = self.inner.add_embeddings(collection, records).await?;
        if !text_records.is_empty() {
            self.text.upsert(collection, text_records).await?;
        }
        Ok(n)
    }

    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.inner
            .query(collection, query_embedding, top_k, filters)
            .await
    }

    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        // 原生 BM25 搜索：先在 tantivy 内应用可表达的过滤，避免 top_k 截断掉有效正文命中。
        let results = self
            .text
            .search_filtered(collection, query_text, top_k, filters.as_ref())
            .await?;
        match filters.as_ref() {
            Some(filter) if requires_precise_bm25_filter(collection, Some(filter)) => {
                filter_bm25_results_by_metadata(self.inner.as_ref(), collection, results, filter)
                    .await
            }
            _ => Ok(results),
        }
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
        _vector_weight: Option<f32>,
    ) -> Result<Vec<SearchResult>, AppError> {
        // 各拉 top_k*2，RRF 融合到 top_k
        let pull = top_k.saturating_mul(2).max(top_k);
        let vector_results = self
            .inner
            .query(collection, query_embedding, pull, filters.clone())
            .await?;
        let mut bm25_results = self
            .text
            .search_filtered(collection, query_text, pull, filters.as_ref())
            .await?;
        if let Some(filter) = filters.as_ref()
            && requires_precise_bm25_filter(collection, Some(filter))
        {
            bm25_results = filter_bm25_results_by_metadata(
                self.inner.as_ref(),
                collection,
                bm25_results,
                filter,
            )
            .await?;
        }
        Ok(rrf_fuse(
            &vector_results,
            &bm25_results,
            top_k,
            RRF_DEFAULT_K,
        ))
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError> {
        let full_collection_wipe = filter.chapter_number.is_none()
            && filter.data_type.is_none()
            && filter.ids.is_empty()
            && filter.id_prefixes.is_empty();
        let ids = filter.ids.clone();
        let id_prefixes = filter.id_prefixes.clone();
        let n = self.inner.delete(collection, filter).await?;
        if full_collection_wipe {
            // 没有细粒度过滤时，清空整段 tantivy 索引保证一致性
            self.text.drop_collection(collection).await?;
        } else {
            // Tantivy 端不保存完整 metadata，精细删除依赖 A4N 稳定 chunk id 前缀。
            if !ids.is_empty() {
                self.text.delete_by_ids(collection, ids).await?;
            }
            if !id_prefixes.is_empty() {
                self.text
                    .delete_by_prefixes(collection, id_prefixes)
                    .await?;
            }
        }
        Ok(n)
    }

    async fn collection_exists(&self, collection: &str) -> Result<bool, AppError> {
        // 双端 OR：任一端存在即视为存在
        Ok(self.inner.collection_exists(collection).await? || self.text.exists(collection))
    }

    async fn create_collection(&self, collection: &str) -> Result<(), AppError> {
        self.inner.create_collection(collection).await?;
        // tantivy 端的 collection 通过首次 upsert 自动创建，这里无需提前创建
        Ok(())
    }

    async fn count(&self, collection: &str) -> Result<usize, AppError> {
        self.inner.count(collection).await
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        self.inner.drop_collection(collection).await?;
        self.text.drop_collection(collection).await
    }

    async fn list_collections(&self) -> Result<Vec<String>, AppError> {
        let mut a = self.inner.list_collections().await?;
        let b = self.text.list_collections();
        for name in b {
            if !a.contains(&name) {
                a.push(name);
            }
        }
        Ok(a)
    }

    async fn update_metadata(
        &self,
        collection: &str,
        id: &str,
        metadata: serde_json::Value,
    ) -> Result<(), AppError> {
        // 元数据语义归 inner 管，tantivy 端只关心 content
        self.inner.update_metadata(collection, id, metadata).await
    }

    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        self.inner.metadata_by_ids(collection, ids).await
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        self.inner.metadata_by_filter(collection, filter).await
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        self.inner.health_check().await
    }

    async fn completeness_report(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        let mut base = self
            .inner
            .completeness_report(collection, expected_count)
            .await?;
        // 用真实 BM25 端数量覆写 actual_bm25
        let bm25_count = self.text.count(collection)?;
        base.actual_bm25 = bm25_count;
        base.is_complete = base.actual_vector >= expected_count
            && bm25_count >= expected_count
            && base.missing_vector_ids.is_empty()
            && base.missing_bm25_ids.is_empty();
        Ok(base)
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        let vector_report = self
            .inner
            .completeness_report_by_ids(collection, expected_ids)
            .await?;
        let bm25_ids = self.text.ids(collection).await?;
        let missing_bm25_ids = expected_ids
            .iter()
            .filter(|id| !bm25_ids.contains(*id))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        Ok(CompletenessReport::from_missing_ids(
            collection.to_string(),
            expected_ids.len(),
            vector_report.actual_vector,
            bm25_ids.len(),
            vector_report.missing_vector_ids,
            missing_bm25_ids,
        ))
    }
}
