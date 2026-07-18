//! LanceDB 向量存储后端
//!
//! 在 `data_dir/lance/` 下维护一个 LanceDB 数据库；每个 collection 对应一张 lance 表。
//! 提供 IVF/HNSW 等 ANN 加速能力，作为当前桌面端持久化向量存储实现。

mod batch;
mod collections;
mod completeness;
mod connection;
mod decode;
mod metadata;
mod search;
mod sql;
mod trait_impl;
mod write;

pub use connection::LanceVectorStore;

#[cfg(test)]
use a4n_rag_core::vector_store::{DeleteFilter, EmbeddingRecord, SearchFilter, VectorStore};

#[cfg(test)]
mod tests;
