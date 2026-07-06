//! VectorStore trait：向量存储与混合搜索抽象
//!
//! 支持纯向量语义搜索、BM25 全文搜索、RRF 混合搜索。
//! 所有向量库后端（LanceDB、pgvector、Qdrant）均实现此 trait。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_core::error::AppError;

/// 嵌入向量记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRecord {
    pub id: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
    /// 原文内容（用于 BM25 全文索引）
    pub content: Option<String>,
}

/// 检索过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    /// 等值过滤
    pub equals: Option<HashMap<String, serde_json::Value>>,
    /// 章节范围过滤
    pub chapter_range: Option<(u32, u32)>,
    /// 数据类型过滤
    pub data_types: Option<Vec<String>>,
}

pub fn metadata_matches_search_filter(metadata: &serde_json::Value, filter: &SearchFilter) -> bool {
    if let Some(equals) = &filter.equals {
        for (key, val) in equals {
            if metadata.get(key) != Some(val) {
                return false;
            }
        }
    }
    if let Some(data_types) = &filter.data_types {
        match metadata.get("data_type").and_then(|v| v.as_str()) {
            Some(dt) if data_types.iter().any(|t| t == dt) => {}
            _ => return false,
        }
    }
    if let Some((min, max)) = filter.chapter_range {
        match metadata.get("chapter_number").and_then(|v| v.as_u64()) {
            Some(ch) if ch <= u64::from(u32::MAX) => {
                let ch = ch as u32;
                if ch < min || ch > max {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// 删除过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFilter {
    pub project_id: i64,
    pub data_type: Option<String>,
    pub chapter_number: Option<u32>,
    /// Delete records with these exact ids.
    #[serde(default)]
    pub ids: Vec<String>,
    /// Delete records whose ids start with any of these prefixes.
    ///
    /// This is intentionally part of the generic filter because the Tantivy
    /// side stores only a compact searchable document, while most A4N RAG
    /// records use stable project-scoped id prefixes.
    #[serde(default)]
    pub id_prefixes: Vec<String>,
}

/// 检索结果（含混合搜索分数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    /// 融合分数（RRF 归一化）
    pub score: f32,
    /// 向量语义相似度
    pub vector_score: Option<f32>,
    /// BM25 文本相关性分数
    pub bm25_score: Option<f32>,
    pub metadata: serde_json::Value,
}

/// 完整性报告
#[derive(Debug, Clone, Serialize)]
pub struct CompletenessReport {
    pub collection: String,
    pub expected: usize,
    pub actual_vector: usize,
    pub actual_bm25: usize,
    pub is_complete: bool,
    pub missing_vector_ids: Vec<String>,
    pub missing_bm25_ids: Vec<String>,
    pub missing_ids: Vec<String>,
}

impl CompletenessReport {
    pub fn count_only(
        collection: impl Into<String>,
        expected: usize,
        actual_vector: usize,
        actual_bm25: usize,
        missing_vector_ids: Vec<String>,
        missing_bm25_ids: Vec<String>,
    ) -> Self {
        let missing_ids = merged_missing_ids(&missing_vector_ids, &missing_bm25_ids);
        Self {
            collection: collection.into(),
            expected,
            actual_vector,
            actual_bm25,
            is_complete: actual_vector >= expected
                && actual_bm25 >= expected
                && missing_vector_ids.is_empty()
                && missing_bm25_ids.is_empty(),
            missing_vector_ids,
            missing_bm25_ids,
            missing_ids,
        }
    }

    pub fn by_expected_ids(
        collection: impl Into<String>,
        expected_ids: &[String],
        actual_vector_ids: &HashSet<String>,
        actual_bm25_ids: Option<&HashSet<String>>,
    ) -> Self {
        let missing_vector_ids = missing_expected_ids(expected_ids, actual_vector_ids);
        let missing_bm25_ids = actual_bm25_ids
            .map(|ids| missing_expected_ids(expected_ids, ids))
            .unwrap_or_default();
        let missing_ids = merged_missing_ids(&missing_vector_ids, &missing_bm25_ids);
        Self {
            collection: collection.into(),
            expected: expected_ids.len(),
            actual_vector: actual_vector_ids.len(),
            actual_bm25: actual_bm25_ids.map(HashSet::len).unwrap_or(0),
            is_complete: missing_vector_ids.is_empty() && missing_bm25_ids.is_empty(),
            missing_vector_ids,
            missing_bm25_ids,
            missing_ids,
        }
    }

    pub fn from_missing_ids(
        collection: impl Into<String>,
        expected: usize,
        actual_vector: usize,
        actual_bm25: usize,
        missing_vector_ids: Vec<String>,
        missing_bm25_ids: Vec<String>,
    ) -> Self {
        let missing_ids = merged_missing_ids(&missing_vector_ids, &missing_bm25_ids);
        Self {
            collection: collection.into(),
            expected,
            actual_vector,
            actual_bm25,
            is_complete: missing_vector_ids.is_empty() && missing_bm25_ids.is_empty(),
            missing_vector_ids,
            missing_bm25_ids,
            missing_ids,
        }
    }
}

fn missing_expected_ids(expected_ids: &[String], actual_ids: &HashSet<String>) -> Vec<String> {
    expected_ids
        .iter()
        .filter(|id| !actual_ids.contains(*id))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn merged_missing_ids(vector_ids: &[String], bm25_ids: &[String]) -> Vec<String> {
    vector_ids
        .iter()
        .chain(bm25_ids)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

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

    /// 检查集合是否存在
    async fn collection_exists(&self, collection: &str) -> Result<bool, AppError>;

    /// 创建集合（同时创建 BM25 索引）
    async fn create_collection(&self, collection: &str) -> Result<(), AppError>;

    /// 获取集合内数据量
    async fn count(&self, collection: &str) -> Result<usize, AppError>;

    /// 删除整个集合
    async fn drop_collection(&self, collection: &str) -> Result<(), AppError>;

    /// 列出所有集合
    async fn list_collections(&self) -> Result<Vec<String>, AppError>;

    /// 更新指定记录的元数据
    async fn update_metadata(
        &self,
        collection: &str,
        id: &str,
        metadata: serde_json::Value,
    ) -> Result<(), AppError>;

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

    /// 健康检查（向量库 + BM25 索引双检）
    async fn health_check(&self) -> Result<bool, AppError>;

    /// 获取集合完整性报告（DB 应有数 vs 实际数）
    async fn completeness_report(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError>;

    /// 获取基于稳定记录 ID 的完整性报告。
    ///
    /// 与 count-only 报告不同，该接口能指出具体缺失的向量端/BM25 端记录。
    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError>;
}
