//! RAG 检索增强服务：向量嵌入 + 持久化向量存储
//!
//! 对接 VectorStore + EmbeddingProvider，实现语义搜索和全文搜索。

mod deletion;
mod embedding;
mod metadata;
mod search;
mod validation;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use a4n_engine::rag::vector_store::VectorStore;

pub use embedding::RagEmbeddedInput;

#[derive(Clone)]
pub struct RagService {
    pub vector_store: Arc<dyn VectorStore>,
}

#[derive(Debug, Clone)]
pub struct RagEmbeddingInput {
    pub chunk_id: String,
    pub text: String,
    pub metadata: serde_json::Value,
}

impl RagService {
    pub fn new(vector_store: Arc<dyn VectorStore>) -> Self {
        Self { vector_store }
    }
}
