use a4n_domain::embedding::embedding_provider_capabilities;
pub(super) use a4n_embedding_contract::{
    EmbedTextsOutput, EmbedTextsPayload, EmbeddingModelPayload,
};
use a4n_embedding_provider_contract::{
    EMBEDDING_CONNECTIVITY_TEST_TEXT, validate_embedding_connectivity_vectors,
};
use a4n_error::AppError;
use a4n_provider_contract::EmbeddingProviderConfig;

pub(super) use crate::dynamic_embedding::{positive_usize, required_text};

pub(super) fn embedding_connectivity_test_payload(
    model: &EmbeddingProviderConfig,
) -> Result<EmbedTextsPayload, AppError> {
    embedding_texts_payload(model, vec![EMBEDDING_CONNECTIVITY_TEST_TEXT.to_string()])
}

pub(super) fn embedding_texts_payload(
    model: &EmbeddingProviderConfig,
    texts: Vec<String>,
) -> Result<EmbedTextsPayload, AppError> {
    let backend = embedding_provider_capabilities(&model.backend).ok_or_else(|| {
        AppError::ConfigError(format!(
            "embedding sidecar 不支持 backend={}",
            model.backend.trim()
        ))
    })?;
    Ok(EmbedTextsPayload {
        backend: backend.backend.to_string(),
        base_url: if backend.requires_base_url {
            Some(required_text(
                model.base_url.as_deref(),
                "Base URL",
                &model.name,
            )?)
        } else {
            None
        },
        api_key: if backend.accepts_api_key {
            model.api_key.clone()
        } else {
            None
        },
        model: required_text(Some(&model.model), "模型名", &model.name)?,
        dimension: positive_usize(model.dimension, "向量维度", &model.name)?,
        batch_size: positive_usize(model.batch_size, "批量大小", &model.name)?,
        texts,
    })
}

pub(super) fn validate_embedding_connectivity_output(
    output: &EmbedTextsOutput,
    expected_dimension: usize,
) -> Result<(), AppError> {
    if output.identity.dimension != expected_dimension {
        return Err(AppError::EmbeddingError(format!(
            "Embedding 连接测试身份维度不匹配：期望 {expected_dimension}，实际 {}",
            output.identity.dimension
        )));
    }
    validate_embedding_connectivity_vectors(&output.embeddings, expected_dimension)
        .map_err(AppError::from)
}

pub(super) fn embedding_model_payload(model: &str) -> EmbeddingModelPayload {
    EmbeddingModelPayload {
        model: model.to_string(),
    }
}

#[cfg(test)]
mod strict_output_tests {
    use super::*;

    #[test]
    fn embedding_output_rejects_unknown_fields() {
        let error = serde_json::from_value::<EmbedTextsOutput>(serde_json::json!({
            "identity": {"backend": "FastEmbed", "model": "model", "dimension": 2},
            "embeddings": [[0.0, 1.0]],
            "legacy_cache_dir": "/tmp/cache"
        }))
        .err()
        .expect("embedding output must reject unknown fields");
        assert!(error.to_string().contains("legacy_cache_dir"));
    }
}
