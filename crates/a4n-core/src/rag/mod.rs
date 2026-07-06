//! RAG 共享契约。

pub mod data_type;

/// 当前全文索引契约版本。
///
/// 该版本进入 RAG metadata，用于在索引策略变化后让同步流程显式重写旧 chunk，
/// 避免内容 hash 未变但 BM25 索引仍停留在旧 schema/分词策略。
pub const RAG_TEXT_INDEX_VERSION: &str = "ngram-v3";

pub use data_type::RagDataType;
