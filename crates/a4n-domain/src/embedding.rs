use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_embedding_contract::EmbeddingModelIdentity;

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
    params.base_url = normalize_optional_config_value(params.base_url);
    params.api_key_encrypted = if capabilities.accepts_api_key {
        normalize_optional_config_value(params.api_key_encrypted)
    } else {
        None
    };
    params.local_model_path = normalize_optional_config_value(params.local_model_path);
    if capabilities.requires_base_url && params.base_url.is_none() {
        return Err("api 嵌入后端必须填写 base_url".into());
    }
    Ok(params)
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
