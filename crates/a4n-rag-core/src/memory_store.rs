//! 内存向量存储：测试用
//!
//! 提供完整的 VectorStore trait 实现，包括向量搜索、BM25 全文搜索和
//! RRF 混合搜索。所有数据存于内存，重启即丢失。

use dashmap::DashMap;

mod bm25;
mod collections;
mod completeness;
mod filters;
mod metadata;
mod records;
mod scoring;
mod search;
#[cfg(test)]
mod tests;
mod trait_impl;
mod write;

use bm25::Bm25Index;
use records::StoredRecord;

/// 内存向量存储，线程安全并支持并发读写。
pub struct MemoryVectorStore {
    collections: DashMap<String, DashMap<String, StoredRecord>>,
    bm25: Bm25Index,
}

impl MemoryVectorStore {
    /// 创建空的内存向量存储。
    pub fn new() -> Self {
        Self {
            collections: DashMap::new(),
            bm25: Bm25Index::new(),
        }
    }
}

impl Default for MemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}
