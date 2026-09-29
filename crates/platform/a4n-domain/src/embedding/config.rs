use crate::config::{
    normalize_config_name, normalize_optional_config_value, positive_i32, required_config_value,
};

use super::{EmbeddingProviderCapabilities, embedding_provider_capabilities};

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
