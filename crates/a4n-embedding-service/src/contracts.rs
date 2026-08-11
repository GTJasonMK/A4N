use a4n_embedding::provider::EmbeddingModelIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EmbedTextsPayload {
    pub(crate) backend: String,
    #[serde(default)]
    pub(crate) base_url: Option<String>,
    #[serde(default)]
    pub(crate) api_key: Option<String>,
    pub(crate) model: String,
    pub(crate) dimension: usize,
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
    pub(crate) model: String,
}

#[cfg(feature = "fastembed")]
impl ModelPayload {
    pub(crate) fn required_model(&self) -> Result<&str, a4n_core::error::AppError> {
        let model = self.model.trim();
        if model.is_empty() {
            return Err(a4n_core::error::AppError::InvalidArgument(
                "embedding 模型操作必须提供模型名".to_string(),
            ));
        }
        Ok(model)
    }
}
