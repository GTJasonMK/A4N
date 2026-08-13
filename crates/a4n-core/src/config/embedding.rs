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

pub const API_EMBEDDING_BACKEND: &str = "api";
pub const FASTEMBED_EMBEDDING_BACKEND: &str = "fastembed";
pub const LOCAL_HASH_EMBEDDING_BACKEND: &str = "local_hash";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingBackendKind {
    Api,
    FastEmbed,
    LocalHash,
}

impl EmbeddingBackendKind {
    pub const fn identity_backend_name(self) -> &'static str {
        match self {
            Self::Api => "Api",
            Self::FastEmbed => "FastEmbed",
            Self::LocalHash => "LocalHash",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddingProviderCapabilities {
    pub backend: &'static str,
    pub display_name: &'static str,
    pub kind: EmbeddingBackendKind,
    pub requires_base_url: bool,
    pub accepts_api_key: bool,
}

pub const EMBEDDING_PROVIDER_CAPABILITIES: &[EmbeddingProviderCapabilities] = &[
    EmbeddingProviderCapabilities {
        backend: API_EMBEDDING_BACKEND,
        display_name: "OpenAI 兼容 API",
        kind: EmbeddingBackendKind::Api,
        requires_base_url: true,
        accepts_api_key: true,
    },
    EmbeddingProviderCapabilities {
        backend: FASTEMBED_EMBEDDING_BACKEND,
        display_name: "FastEmbed 本地模型",
        kind: EmbeddingBackendKind::FastEmbed,
        requires_base_url: false,
        accepts_api_key: false,
    },
    EmbeddingProviderCapabilities {
        backend: LOCAL_HASH_EMBEDDING_BACKEND,
        display_name: "本地哈希（开发）",
        kind: EmbeddingBackendKind::LocalHash,
        requires_base_url: false,
        accepts_api_key: false,
    },
];

pub fn embedding_provider_capabilities(value: &str) -> Option<EmbeddingProviderCapabilities> {
    let value = value.trim();
    EMBEDDING_PROVIDER_CAPABILITIES
        .iter()
        .copied()
        .find(|capabilities| capabilities.backend.eq_ignore_ascii_case(value))
}

pub fn validate_embedding_config_params(
    mut params: CreateEmbeddingConfigParams,
) -> Result<CreateEmbeddingConfigParams, AppError> {
    params.name = normalize_config_name(params.name, "嵌入配置")?;
    let capabilities = required_embedding_provider_capabilities(&params.backend)?;
    params.backend = capabilities.backend.to_string();
    params.model = required_config_value(&params.model, "嵌入配置", "model")?;
    params.dimension = positive_i32(params.dimension, "嵌入配置 dimension")?;
    params.batch_size = positive_i32(params.batch_size, "嵌入配置 batch_size")?;
    params.base_url = normalize_optional_config_value(params.base_url);
    params.api_key_encrypted = if capabilities.accepts_api_key {
        normalize_optional_config_value(params.api_key_encrypted)
    } else {
        None
    };
    params.local_model_path = normalize_optional_config_value(params.local_model_path);
    if capabilities.requires_base_url && params.base_url.is_none() {
        return Err(AppError::InvalidArgument(
            "api 嵌入后端必须填写 base_url".into(),
        ));
    }
    Ok(params)
}

pub fn validate_embedding_config_model(
    model: &embedding_config::Model,
) -> Result<CreateEmbeddingConfigParams, AppError> {
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
}

fn required_embedding_provider_capabilities(
    value: &str,
) -> Result<EmbeddingProviderCapabilities, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument("嵌入后端不能为空".into()));
    }
    embedding_provider_capabilities(value)
        .ok_or_else(|| AppError::InvalidArgument(format!("不支持的嵌入后端: {value}")))
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
    fn normalizes_supported_local_backends_and_clears_irrelevant_api_keys() {
        for capabilities in EMBEDDING_PROVIDER_CAPABILITIES
            .iter()
            .filter(|capabilities| !capabilities.accepts_api_key)
        {
            let backend = match capabilities.kind {
                EmbeddingBackendKind::FastEmbed => " FastEmbed ",
                EmbeddingBackendKind::LocalHash => "LOCAL_HASH",
                EmbeddingBackendKind::Api => unreachable!(),
            };
            let params = validate_embedding_config_params(params(backend))
                .expect("supported local embedding backend");
            assert_eq!(params.backend, capabilities.backend);
            assert_eq!(params.name, "默认嵌入");
            assert_eq!(params.model, "embedding-model");
            assert_eq!(params.api_key_encrypted, None);
            assert_eq!(
                params.local_model_path.as_deref(),
                Some("/models/embedding")
            );
        }
    }

    #[test]
    fn api_backend_requires_and_normalizes_endpoint() {
        assert_eq!(
            validate_embedding_config_params(params(API_EMBEDDING_BACKEND))
                .expect_err("api endpoint must be explicit")
                .to_string(),
            "参数无效：api 嵌入后端必须填写 base_url"
        );

        let mut params = params(API_EMBEDDING_BACKEND);
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

        let mut invalid_dimension = params(FASTEMBED_EMBEDDING_BACKEND);
        invalid_dimension.dimension = 0;
        assert_eq!(
            validate_embedding_config_params(invalid_dimension)
                .expect_err("positive dimension")
                .to_string(),
            "参数无效：嵌入配置 dimension 必须大于 0"
        );

        let mut invalid_batch = params(FASTEMBED_EMBEDDING_BACKEND);
        invalid_batch.batch_size = 0;
        assert_eq!(
            validate_embedding_config_params(invalid_batch)
                .expect_err("positive batch size")
                .to_string(),
            "参数无效：嵌入配置 batch_size 必须大于 0"
        );
    }
}
