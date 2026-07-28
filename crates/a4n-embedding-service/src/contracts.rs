use a4n_embedding::provider::EmbeddingModelIdentity;
use serde::{Deserialize, Serialize};

#[cfg(feature = "fastembed")]
use a4n_embedding_fastembed::DEFAULT_FASTEMBED_MODEL;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EmbedTextsPayload {
    pub(crate) backend: String,
    pub(crate) model: String,
    pub(crate) dimension: usize,
    #[serde(default = "default_batch_size")]
    pub(crate) batch_size: usize,
    pub(crate) texts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct EmbedTextsOutput {
    pub(crate) identity: EmbeddingModelIdentity,
    pub(crate) embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "fastembed")]
pub(crate) struct ModelPayload {
    pub(crate) operation: String,
    #[serde(default = "default_fastembed_model")]
    pub(crate) model: String,
}

#[cfg(feature = "fastembed")]
impl ModelPayload {
    pub(crate) fn validate_operation(
        &self,
        expected: &str,
    ) -> Result<(), a4n_core::error::AppError> {
        let operation = self.operation.trim();
        if operation == expected {
            return Ok(());
        }
        Err(a4n_core::error::AppError::InvalidArgument(format!(
            "embedding 模型端点要求 operation={expected}，实际为 {operation}"
        )))
    }
}

#[cfg(feature = "fastembed")]
fn default_fastembed_model() -> String {
    DEFAULT_FASTEMBED_MODEL.to_string()
}

fn default_batch_size() -> usize {
    32
}
