//! Persistent RAG index implementations.
//!
//! This crate owns the heavy LanceDB/Tantivy dependencies. Host code should
//! depend on it only in full or embedded-index profiles.

pub mod hybrid_store;
pub mod lance_store;
pub mod tantivy_index;
