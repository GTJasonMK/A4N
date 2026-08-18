mod api;
mod contracts;
mod embedded;
mod provider;
mod records;

pub use contracts::RagEmbeddedInput;
pub(crate) use contracts::RagEmbeddingInput;

#[cfg(test)]
mod tests;

pub(crate) use embedded::store_embedded_rag_records_in_store;
pub(crate) use provider::{
    embed_and_store_rag_record_in_store, embed_and_store_rag_records_in_store,
};
