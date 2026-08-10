use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 嵌入向量记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingRecord {
    pub id: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
    /// 原文内容（用于 BM25 全文索引）
    pub content: String,
}

/// 检索过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchFilter {
    /// 等值过滤
    pub equals: Option<HashMap<String, serde_json::Value>>,
    /// 章节范围过滤
    pub chapter_range: Option<(u32, u32)>,
    /// 数据类型过滤
    pub data_types: Option<Vec<String>>,
}

/// 删除过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
