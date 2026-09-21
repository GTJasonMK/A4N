//! Persistent RAG index implementations backed by Qdrant.
//!
//! Host code should depend on this crate only in full / embedded-rag profiles.

mod point_id;
mod qdrant_store;
mod sparse_text;

pub use qdrant_store::{QdrantStoreConfig, QdrantVectorStore};
