use a4n_core::{
    config::embedding_provider_capabilities, error::AppError, models::ai::embedding_config,
};
use a4n_embedding::provider::{EMBEDDING_CONNECTIVITY_TEST_TEXT, EmbeddingModelIdentity};
use serde::{Deserialize, Serialize};

pub(super) use crate::dynamic_embedding::{positive_usize, required_text};

pub(super) fn embedding_connectivity_test_payload(
    model: &embedding_config::Model,
) -> Result<EmbeddingTextsPayload, AppError> {
    embedding_texts_payload(model, vec![EMBEDDING_CONNECTIVITY_TEST_TEXT.to_string()])
}

pub(super) fn embedding_texts_payload(
    model: &embedding_config::Model,
    texts: Vec<String>,
) -> Result<EmbeddingTextsPayload, AppError> {
    let backend = embedding_provider_capabilities(&model.backend).ok_or_else(|| {
        AppError::ConfigError(format!(
            "embedding sidecar 不支持 backend={}",
            model.backend.trim()
        ))
    })?;
    Ok(EmbeddingTextsPayload {
        backend: backend.backend.to_string(),
        base_url: if backend.requires_base_url {
            Some(required_text(
                model.base_url.as_deref().unwrap_or_default(),
                "Base URL",
                &model.name,
            )?)
        } else {
            None
        },
        api_key: if backend.accepts_api_key {
            model.api_key_encrypted.clone()
        } else {
            None
        },
        model: required_text(&model.model, "模型名", &model.name)?,
        dimension: positive_usize(model.dimension, "向量维度", &model.name)?,
        batch_size: positive_usize(model.batch_size, "批量大小", &model.name)?,
        texts,
    })
}

pub(super) fn validate_embedding_connectivity_output(
    output: &EmbeddingTextsOutput,
    expected_dimension: usize,
) -> Result<(), AppError> {
    if output.identity.dimension != expected_dimension {
        return Err(AppError::EmbeddingError(format!(
            "Embedding 连接测试身份维度不匹配：期望 {expected_dimension}，实际 {}",
            output.identity.dimension
        )));
    }
    if output.embeddings.len() != 1 {
        return Err(AppError::EmbeddingError(format!(
            "Embedding 连接测试必须返回 1 个向量，实际返回 {} 个",
            output.embeddings.len()
        )));
    }
    let actual_dimension = output.embeddings[0].len();
    if actual_dimension != expected_dimension {
        return Err(AppError::EmbeddingError(format!(
            "Embedding 连接测试向量维度不匹配：期望 {expected_dimension}，实际 {actual_dimension}"
        )));
    }
    Ok(())
}

pub(super) fn embedding_model_payload(model: &str) -> ModelPayload {
    ModelPayload {
        model: model.to_string(),
    }
}

#[derive(Debug, Serialize)]
pub(super) struct EmbeddingTextsPayload {
    pub(super) backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) api_key: Option<String>,
    pub(super) model: String,
    pub(super) dimension: usize,
    pub(super) batch_size: usize,
    pub(super) texts: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ModelPayload {
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
