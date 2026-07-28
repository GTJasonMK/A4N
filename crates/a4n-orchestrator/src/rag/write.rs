mod api;
mod contracts;
pub mod embedded;
pub mod provider;
pub mod records;

pub use contracts::{RagEmbeddedInput, RagEmbeddingInput};

#[cfg(test)]
mod tests;

pub use embedded::store_embedded_rag_records_in_store;

// embed_and_store_* 由 `crate::rag` 与 `context/rag_store` 直接从 `write::provider` 引用。
