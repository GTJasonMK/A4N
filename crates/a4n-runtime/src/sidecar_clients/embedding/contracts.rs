use a4n_core::error::AppError;
use a4n_core::models::ai::embedding_config;
use a4n_embedding::provider::EmbeddingModelIdentity;
use serde::{Deserialize, Serialize};

pub(super) use crate::dynamic_embedding::{positive_usize, required_text};

pub(super) fn embedding_connectivity_test_payload(
    model: &embedding_config::Model,
    text: &str,
) -> Result<EmbeddingTextsPayload, AppError> {
    Ok(EmbeddingTextsPayload {
        backend: model.backend.trim().to_ascii_lowercase(),
        base_url: model.base_url.clone(),
        api_key: model.api_key_encrypted.clone(),
        model: required_text(&model.model, "模型名", &model.name)?,
        dimension: positive_usize(model.dimension, "向量维度", &model.name)?,
        batch_size: positive_usize(model.batch_size, "批量大小", &model.name)?,
        texts: vec![text.to_string()],
    })
}

pub(super) fn embedding_connectivity_output_matches(
    output: &EmbeddingTextsOutput,
    expected_dimension: usize,
) -> bool {
    output.identity.dimension == expected_dimension
        && output.embeddings.len() == 1
        && output.embeddings[0].len() == expected_dimension
}

pub(super) fn embedding_model_operation_payload(
    operation: &str,
    model: &str,
) -> ModelOperationPayload {
    ModelOperationPayload {
        operation: operation.to_string(),
        model: model.to_string(),
    }
}

#[derive(Debug, Serialize)]
pub(super) struct EmbeddingTextsPayload {
    pub(super) backend: String,
    pub(super) base_url: Option<String>,
    pub(super) api_key: Option<String>,
    pub(super) model: String,
    pub(super) dimension: usize,
    pub(super) batch_size: usize,
    pub(super) texts: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ModelOperationPayload {
    pub(super) operation: String,
    pub(super) model: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmbeddingTextsOutput {
    pub(super) identity: EmbeddingModelIdentity,
    pub(super) embeddings: Vec<Vec<f32>>,
}

#[cfg(test)]
mod strict_output_tests {
    use super::*;

    #[test]
    fn embedding_output_rejects_unknown_fields() {
        let error = serde_json::from_value::<EmbeddingTextsOutput>(serde_json::json!({
            "identity": {"backend": "FastEmbed", "model": "model", "dimension": 2},
            "embeddings": [[0.0, 1.0]],
            "legacy_cache_dir": "/tmp/cache"
        }))
        .expect_err("embedding output must reject unknown fields");
        assert!(error.to_string().contains("legacy_cache_dir"));
    }
}
