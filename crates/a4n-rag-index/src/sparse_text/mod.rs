//! 中文友好的字符 n-gram + BM25 sparse 嵌入。
//!
//! 对齐原 Tantivy `NgramTokenizer(2,3)` 的意图：无空格中文靠 bigram/trigram 检索。
//! 算法参数固定，升级须同步 bump 索引格式并 rebuild。

mod bm25;
mod tokenize;

pub use bm25::{SparseEmbedding, bm25_document, bm25_query};
pub(crate) use tokenize::tokenize_for_sparse;

#[cfg(test)]
mod tests;
