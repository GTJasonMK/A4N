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
    /// Delete records with these exact ids.
    #[serde(default)]
    pub ids: Vec<String>,
    /// Delete records whose ids start with any of these prefixes.
    #[serde(default)]
    pub id_prefixes: Vec<String>,
}

/// 检索结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    /// 当前检索实现给出的排序分数。
    pub score: f32,
    pub metadata: serde_json::Value,
}
