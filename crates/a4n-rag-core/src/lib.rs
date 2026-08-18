//! RAG contracts and lightweight in-memory utilities.
//!
//! Persistent index implementations are intentionally kept in
//! `a4n-rag-index` so host minimal builds can depend on these contracts
//! without compiling persistent index backends.

pub const DEFAULT_QDRANT_COLLECTION: &str = "a4n_rag";

pub mod chunk_strategy;
pub mod errors;
pub mod fusion;
pub mod memory_store;
pub mod metadata;
pub mod record_text;
pub mod semantic_chunker;
pub mod validation;
pub mod vector_store;
