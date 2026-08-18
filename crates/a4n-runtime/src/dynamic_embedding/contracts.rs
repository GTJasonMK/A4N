use std::path::PathBuf;

use a4n_core::config::{app_config, embedding_provider_capabilities};
use a4n_core::error::AppError;
use a4n_core::models::ai::embedding_config;
use a4n_embedding::provider::EmbeddingModelIdentity;

pub fn model_identity_from_config(
    model: &embedding_config::Model,
) -> Result<EmbeddingModelIdentity, AppError> {
    let backend = embedding_provider_capabilities(&model.backend).ok_or_else(|| {
        AppError::ConfigError(format!(
            "嵌入配置 {} 使用不支持的后端: {}",
            model.name, model.backend
        ))
    })?;
    let dimension = positive_usize(model.dimension, "向量维度", &model.name)?;
    let model_name = required_text(Some(&model.model), "模型名", &model.name)?;
    Ok(EmbeddingModelIdentity {
        backend: backend.kind.identity_backend_name().to_string(),
        model: model_name,
        dimension,
    })
}

pub(crate) fn resolve_fastembed_cache_dir(
    value: Option<&str>,
    default_cache_dir: PathBuf,
) -> Result<PathBuf, AppError> {
    match value {
        Some(value) => {
            let value = value.trim();
            if value.is_empty() {
                return Err(AppError::ConfigError(
                    "fastembed 本地模型缓存路径不能为空".to_string(),
                ));
            }
            Ok(app_config::resolve_portable_path(value))
        }
        None => Ok(default_cache_dir),
    }
}

pub(crate) fn positive_usize(value: i32, label: &str, name: &str) -> Result<usize, AppError> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::ConfigError(format!("嵌入配置 {name} 的{label}必须为正整数")))
}

pub(crate) fn required_text(
    value: Option<&str>,
    label: &str,
    name: &str,
) -> Result<String, AppError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| AppError::ConfigError(format!("嵌入配置 {name} 缺少{label}")))
}

pub(super) struct ResolvedEmbeddingConfig {
    pub(super) name: String,
    pub(super) backend: String,
    pub(super) base_url: Option<String>,
    pub(super) api_key: Option<String>,
    pub(super) model: String,
    pub(super) dimension: usize,
    pub(super) batch_size: usize,
    pub(super) fastembed_cache_dir: PathBuf,
}
