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
    pub(crate) operation: String,
    pub(crate) model: Option<String>,
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
    pub(crate) fn required_model(&self) -> Result<&str, a4n_core::error::AppError> {
        self.model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .ok_or_else(|| {
                a4n_core::error::AppError::InvalidArgument(
                    "embedding 模型操作必须提供模型名".to_string(),
                )
            })
    }
}
