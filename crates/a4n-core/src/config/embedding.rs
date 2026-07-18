use crate::{error::AppError, models::ai::embedding_config};

use super::{
    normalize_config_name, normalize_optional_config_value, positive_i32, required_config_value,
};

#[derive(Debug, Clone)]
pub struct CreateEmbeddingConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateEmbeddingConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
}

pub fn validate_embedding_config_params(
    mut params: CreateEmbeddingConfigParams,
) -> Result<CreateEmbeddingConfigParams, AppError> {
    params.name = normalize_config_name(params.name, "嵌入配置")?;
    params.backend = normalize_embedding_backend(&params.backend)?;
    params.model = required_config_value(&params.model, "嵌入配置", "model")?;
    params.dimension = positive_i32(params.dimension, "嵌入配置 dimension")?;
    params.batch_size = positive_i32(params.batch_size, "嵌入配置 batch_size")?;
    params.base_url = normalize_optional_config_value(params.base_url);
    params.api_key_encrypted = normalize_optional_config_value(params.api_key_encrypted);
    params.local_model_path = normalize_optional_config_value(params.local_model_path);
    ensure_embedding_backend_endpoint(&params.backend, params.base_url.as_deref())?;
    Ok(params)
}

pub fn validate_embedding_config_model(model: &embedding_config::Model) -> Result<(), AppError> {
    validate_embedding_config_params(CreateEmbeddingConfigParams {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path.clone(),
    })
    .map(|_| ())
}

fn normalize_embedding_backend(value: &str) -> Result<String, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "api" => Ok("api".to_string()),
        "fastembed" => Ok("fastembed".to_string()),
        "local_hash" => Ok("local_hash".to_string()),
        "" => Err(AppError::InvalidArgument("嵌入后端不能为空".into())),
        other => Err(AppError::InvalidArgument(format!(
            "不支持的嵌入后端: {other}"
        ))),
    }
}

fn ensure_embedding_backend_endpoint(
    backend: &str,
    base_url: Option<&str>,
) -> Result<(), AppError> {
    if backend == "api" && base_url.is_none() {
        return Err(AppError::InvalidArgument(
            "api 嵌入后端必须填写 base_url".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(backend: &str) -> CreateEmbeddingConfigParams {
        CreateEmbeddingConfigParams {
            name: "  默认嵌入  ".to_string(),
            backend: backend.to_string(),
            base_url: None,
            api_key_encrypted: Some("  secret  ".to_string()),
            model: "  embedding-model  ".to_string(),
            dimension: 384,
            batch_size: 32,
            local_model_path: Some("  /models/embedding  ".to_string()),
        }
    }

    #[test]
    fn normalizes_supported_local_backends_and_optional_fields() {
        for backend in [" FastEmbed ", "LOCAL_HASH"] {
            let params = validate_embedding_config_params(params(backend))
                .expect("supported local embedding backend");
            assert!(matches!(
                params.backend.as_str(),
                "fastembed" | "local_hash"
            ));
            assert_eq!(params.name, "默认嵌入");
            assert_eq!(params.model, "embedding-model");
            assert_eq!(params.api_key_encrypted.as_deref(), Some("secret"));
            assert_eq!(
                params.local_model_path.as_deref(),
                Some("/models/embedding")
            );
        }
    }

    #[test]
    fn api_backend_requires_and_normalizes_endpoint() {
        assert_eq!(
            validate_embedding_config_params(params("api"))
                .expect_err("api endpoint must be explicit")
                .to_string(),
            "参数无效：api 嵌入后端必须填写 base_url"
        );

        let mut params = params("api");
        params.base_url = Some("  https://embedding.example/v1  ".to_string());
        assert_eq!(
            validate_embedding_config_params(params)
                .expect("api endpoint provided")
                .base_url
                .as_deref(),
            Some("https://embedding.example/v1")
        );
    }

    #[test]
    fn rejects_empty_unknown_backend_and_non_positive_limits() {
        assert_eq!(
            validate_embedding_config_params(params(" "))
                .expect_err("empty backend")
                .to_string(),
            "参数无效：嵌入后端不能为空"
        );
        assert_eq!(
            validate_embedding_config_params(params("embedded"))
                .expect_err("unsupported backend")
                .to_string(),
            "参数无效：不支持的嵌入后端: embedded"
        );

        let mut invalid_dimension = params("fastembed");
        invalid_dimension.dimension = 0;
        assert_eq!(
            validate_embedding_config_params(invalid_dimension)
                .expect_err("positive dimension")
                .to_string(),
            "参数无效：嵌入配置 dimension 必须大于 0"
        );

        let mut invalid_batch = params("fastembed");
        invalid_batch.batch_size = 0;
        assert_eq!(
            validate_embedding_config_params(invalid_batch)
                .expect_err("positive batch size")
                .to_string(),
            "参数无效：嵌入配置 batch_size 必须大于 0"
        );
    }
}
