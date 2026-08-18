use a4n_domain::embedding::EmbeddingModelIdentity;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedTextsPayload {
    pub backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub model: String,
    pub dimension: usize,
    pub batch_size: usize,
    pub texts: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedTextsOutput {
    pub identity: EmbeddingModelIdentity,
    pub embeddings: Vec<Vec<f32>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingModelPayload {
    pub model: String,
}
