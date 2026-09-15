//! Serialized contracts for the embedding sidecar.
//!
//! Provider implementations, model storage, and sidecar transport stay
//! outside this crate so clients can depend on the wire types alone.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const EMBEDDING_SERVICE_NAME: &str = "embedding";
pub const EMBEDDING_CAPABILITY_VERSION: &str = "embedding-v5";
pub const EMBEDDING_SERVICE_ADDR_ENV: &str = "A4N_EMBEDDING_SERVICE_ADDR";
pub const EMBEDDING_SERVICE_DEFAULT_ADDR: &str = "127.0.0.1:47643";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingModelIdentity {
    pub backend: String,
    pub model: String,
    #[schemars(with = "u32")]
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmbedTextsOutput {
    pub identity: EmbeddingModelIdentity,
    pub embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingModelPayload {
    pub model: String,
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
    fn embedding_contract_rejects_unknown_fields() {
        let error = serde_json::from_value::<EmbedTextsPayload>(serde_json::json!({
            "backend": "local_hash",
            "base_url": null,
            "api_key": null,
            "model": "local-sha256-hash",
            "dimension": 384,
            "batch_size": 8,
            "texts": [],
            "legacy_field": true
        }))
        .expect_err("unknown embedding payload fields must fail");
        assert!(error.to_string().contains("legacy_field"));
    }

    #[test]
    fn embedding_identity_rejects_unknown_fields() {
        let error = serde_json::from_value::<EmbeddingModelIdentity>(serde_json::json!({
            "backend": "FastEmbed",
            "model": "model",
            "dimension": 384,
            "legacy_backend": "local"
        }))
        .expect_err("unknown embedding identity fields must fail");
        assert!(error.to_string().contains("legacy_backend"));
    }

    #[test]
    fn embedding_output_rejects_unknown_fields() {
        let error = serde_json::from_value::<EmbedTextsOutput>(serde_json::json!({
            "identity": {"backend": "LocalHash", "model": "local", "dimension": 2},
            "embeddings": [[0.0, 1.0]],
            "legacy_field": true
        }))
        .expect_err("unknown embedding output fields must fail");
        assert!(error.to_string().contains("legacy_field"));
    }
}
