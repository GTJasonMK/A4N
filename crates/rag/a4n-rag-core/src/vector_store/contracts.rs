use serde::{Deserialize, Serialize};

pub use a4n_rag_value_contract::vector_store::{SearchFilter, SearchResult};

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
