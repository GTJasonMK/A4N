#[derive(Debug, Clone)]
pub(crate) struct RagEmbeddingInput {
    pub(crate) chunk_id: String,
    pub(crate) text: String,
    pub(crate) metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RagEmbeddedInput {
    pub chunk_id: String,
    pub text: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
}
