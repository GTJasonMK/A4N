use super::{CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult};
use a4n_core::error::AppError;
use async_trait::async_trait;
use std::collections::HashMap;

/// VectorStore trait
///
/// 向量存储 + 全文索引的统一抽象。所有操作同时维护向量库和 BM25 索引，
/// 保证双端数据一致性。
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// 批量添加嵌入向量（同时更新 BM25 索引）
    async fn add_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError>;

    /// 纯向量语义搜索
    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError>;

    /// BM25 全文搜索
    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError>;

    /// 混合搜索（向量 + BM25，使用 RRF 融合排序）
    ///
    /// `vector_weight` 控制向量搜索结果在融合中的权重，默认 0.5。
    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
        vector_weight: Option<f32>,
    ) -> Result<Vec<SearchResult>, AppError>;

    /// 条件删除（同时清理 BM25 索引）
    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError>;

    /// 删除整个集合
    async fn drop_collection(&self, collection: &str) -> Result<(), AppError>;

    /// 按稳定记录 ID 批量读取元数据。
    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, AppError>;

    /// 按删除过滤条件读取元数据，用于增量同步前比对已入库记录。
    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError>;

    /// 获取基于稳定记录 ID 的完整性报告。
    /// 该接口能指出具体缺失的向量端/BM25 端记录。
    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError>;
}
