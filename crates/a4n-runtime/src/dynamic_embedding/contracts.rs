use std::path::PathBuf;

use a4n_core::config::app_config::{self, AppConfig};
use a4n_core::error::AppError;
use a4n_core::models::ai::embedding_config;
use a4n_embedding::provider::EmbeddingModelIdentity;

pub fn default_fastembed_cache_dir(config: &AppConfig) -> PathBuf {
    config.cache_dir.join("embedding-models")
}

pub fn effective_fastembed_cache_dir(default_cache_dir: PathBuf) -> PathBuf {
    app_config::resolve_portable_path(default_cache_dir)
}

pub fn model_identity_from_config(
    model: &embedding_config::Model,
) -> Result<EmbeddingModelIdentity, AppError> {
    let backend = SupportedEmbeddingBackend::parse(&model.backend).ok_or_else(|| {
        AppError::ConfigError(format!(
            "嵌入配置 {} 使用不支持的后端: {}",
            model.name, model.backend
        ))
    })?;
    let dimension = positive_usize(model.dimension, "向量维度", &model.name)?;
    let model_name = required_text(&model.model, "模型名", &model.name)?;
    Ok(EmbeddingModelIdentity {
        backend: backend.identity_backend_name().to_string(),
        model: model_name,
        dimension,
    })
}

pub(crate) fn resolve_fastembed_cache_dir(
    value: Option<&str>,
    default_cache_dir: PathBuf,
) -> PathBuf {
    let cache_dir = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(default_cache_dir);
    effective_fastembed_cache_dir(cache_dir)
}

pub(crate) fn positive_usize(value: i32, label: &str, name: &str) -> Result<usize, AppError> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::ConfigError(format!("嵌入配置 {name} 的{label}必须为正整数")))
}

pub(crate) fn required_text(value: &str, label: &str, name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::ConfigError(format!(
            "嵌入配置 {name} 缺少{label}"
        )))
    } else {
        Ok(value.to_string())
    }
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

pub(super) enum SupportedEmbeddingBackend {
    Api,
    LocalHash,
    FastEmbed,
}

impl SupportedEmbeddingBackend {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "api" => Some(Self::Api),
            "local_hash" => Some(Self::LocalHash),
            "fastembed" => Some(Self::FastEmbed),
            _ => None,
        }
    }

    fn identity_backend_name(&self) -> &'static str {
        match self {
            Self::Api => "Api",
            Self::LocalHash => "LocalHash",
            Self::FastEmbed => "FastEmbed",
        }
    }
}
