//! Persistent RAG index implementations backed by Qdrant.
//!
//! Host code should depend on this crate only in full / embedded-rag profiles.

pub mod point_id;
pub mod qdrant_store;
pub mod sparse_text;

pub use qdrant_store::{QdrantStoreConfig, QdrantVectorStore};
