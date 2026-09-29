use super::config::{
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

const API_EMBEDDING_BACKEND: &str = "api";
const FASTEMBED_EMBEDDING_BACKEND: &str = "fastembed";
const LOCAL_HASH_EMBEDDING_BACKEND: &str = "local_hash";

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
) -> Result<CreateEmbeddingConfigParams, String> {
    params.name = normalize_config_name(params.name, "嵌入配置")?;
    let capabilities = required_embedding_provider_capabilities(&params.backend)?;
    params.backend = capabilities.backend.to_string();
    params.model = required_config_value(&params.model, "嵌入配置", "model")?;
    params.dimension = positive_i32(params.dimension, "嵌入配置 dimension")?;
    params.batch_size = positive_i32(params.batch_size, "嵌入配置 batch_size")?;
    params.api_key_encrypted = if capabilities.accepts_api_key {
        normalize_optional_config_value(params.api_key_encrypted)
    } else {
        None
    };
    let base_url = normalize_optional_config_value(params.base_url);
    params.base_url = if capabilities.requires_base_url {
        let base_url = base_url.ok_or_else(|| "api 嵌入后端必须填写 base_url".to_string())?;
        Some(crate::config::validate_http_base_url(
            &base_url,
            "嵌入配置",
            params.api_key_encrypted.as_deref(),
        )?)
    } else {
        base_url
    };
    params.local_model_path = normalize_optional_config_value(params.local_model_path);
    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::{CreateEmbeddingConfigParams, validate_embedding_config_params};

    fn api_config(base_url: &str, api_key: Option<&str>) -> CreateEmbeddingConfigParams {
        CreateEmbeddingConfigParams {
            name: "test embedding".to_string(),
            backend: "api".to_string(),
            base_url: Some(base_url.to_string()),
            api_key_encrypted: api_key.map(str::to_string),
            model: "embedding-model".to_string(),
            dimension: 384,
            batch_size: 32,
            local_model_path: None,
        }
    }

    #[test]
    fn api_embedding_requires_a_valid_endpoint_and_protects_credentials() {
        assert!(
            validate_embedding_config_params(api_config("not-a-url", None))
                .unwrap_err()
                .contains("不是有效 URL")
        );
        assert!(
            validate_embedding_config_params(api_config("http://api.example/v1", Some("secret")))
                .unwrap_err()
                .contains("必须使用 HTTPS")
        );
        assert!(
            validate_embedding_config_params(api_config(
                "http://127.0.0.1:8080/v1",
                Some("secret")
            ))
            .is_ok()
        );
    }
}

fn required_embedding_provider_capabilities(
    value: &str,
) -> Result<EmbeddingProviderCapabilities, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("嵌入后端不能为空".into());
    }
    embedding_provider_capabilities(value).ok_or_else(|| format!("不支持的嵌入后端: {value}"))
}
