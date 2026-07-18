#[derive(Debug, Clone)]
pub struct RagEmbeddingInput {
    pub chunk_id: String,
    pub text: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RagEmbeddedInput {
    pub chunk_id: String,
    pub text: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
}
