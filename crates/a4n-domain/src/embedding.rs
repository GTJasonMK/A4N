use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingModelIdentity {
    pub backend: String,
    pub model: String,
    #[schemars(with = "u32")]
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocalEmbeddingModelSummary {
    pub name: String,
    pub model_code: String,
    pub description: String,
    pub dimension: usize,
    pub downloaded: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingModelDownloadInfoSummary {
    pub model_name: String,
    pub model_code: String,
    pub total_size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub missing_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_identity_rejects_unknown_fields() {
        let error = serde_json::from_value::<EmbeddingModelIdentity>(serde_json::json!({
            "backend": "FastEmbed",
            "model": "model",
            "dimension": 384,
            "legacy_backend": "local"
        }))
        .err()
        .expect("embedding identity must reject unknown fields");
        assert!(error.to_string().contains("legacy_backend"));
    }
}
