mod api;
mod embedded;
mod provider;
mod records;

#[derive(Debug, Clone)]
pub(crate) struct RagEmbeddingInput {
    pub(crate) chunk_id: String,
    pub(crate) text: String,
    pub(crate) metadata: serde_json::Value,
}

#[cfg(test)]
mod tests;

pub(crate) use embedded::store_embedded_rag_records_in_store;
pub(crate) use provider::embed_rag_records;
