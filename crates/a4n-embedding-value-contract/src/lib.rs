//! Stable embedding identity values shared by RAG and provider contracts.
//!
//! Wire payloads and sidecar service settings remain in
//! `a4n-embedding-contract` so algorithm crates do not inherit transport DTOs.

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
        .expect_err("unknown embedding identity fields must fail");
        assert!(error.to_string().contains("legacy_backend"));
    }
}
