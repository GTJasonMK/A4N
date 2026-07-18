//! Tantivy 全文索引。
//!
//! 每个 collection 使用独立索引目录，写入和搜索的阻塞操作均在
//! `spawn_blocking` 中执行，写入后 reload 长生命周期 reader。

mod collection;
mod delete;
mod document;
mod management;
mod query;
mod search;
mod state;
#[cfg(test)]
mod tests;
mod write;

pub use state::{TantivyTextIndex, TantivyTextRecord};

pub(super) const CONTENT_NGRAM_TOKENIZER: &str = "a4n_content_ngram_2_3";
pub(super) const FIELD_CHAPTER_NUMBER: &str = "chapter_number";
pub(super) const FIELD_PROJECT_ID: &str = "project_id";
pub(super) const QUERY_NGRAM_LIMIT: usize = 384;
