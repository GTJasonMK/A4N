//! VectorStore trait：向量存储与混合搜索抽象
//!
//! 支持纯向量语义搜索、BM25 全文搜索、RRF 混合搜索。
//! 所有向量库后端（LanceDB、pgvector、Qdrant）均实现此 trait。

mod completeness;
mod contracts;
pub(crate) mod filters;
mod store;

pub use a4n_rag_value_contract::vector_store::{SearchFilter, SearchResult};
pub use completeness::CompletenessReport;
pub use contracts::{DeleteFilter, EmbeddingRecord};
pub use store::VectorStore;

#[cfg(test)]
mod tests;
