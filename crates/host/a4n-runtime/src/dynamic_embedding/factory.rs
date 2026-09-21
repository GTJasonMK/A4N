use std::path::PathBuf;
use std::sync::Arc;

use a4n_domain::embedding::{EmbeddingBackendKind, embedding_provider_capabilities};
#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::FastEmbedProvider;
use a4n_embedding_provider_contract::EmbeddingProvider;
use a4n_embedding_providers::http::HttpEmbeddingProvider;
use a4n_embedding_providers::local::LocalEmbeddingProvider;
use a4n_error::AppError;
use a4n_provider_contract::EmbeddingProviderConfig;

use super::contracts::{
    ResolvedEmbeddingConfig, positive_usize, required_text, resolve_fastembed_cache_dir,
};

pub(crate) fn make_provider_from_model(
    model: &EmbeddingProviderConfig,
    embedding_models_dir: PathBuf,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let dimension = positive_usize(model.dimension, "向量维度", &model.name)?;
    let batch_size = positive_usize(model.batch_size, "批量大小", &model.name)?;
    let fastembed_cache_dir =
        resolve_fastembed_cache_dir(model.local_model_path.as_deref(), embedding_models_dir)?;
    make_provider(ResolvedEmbeddingConfig {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key: model.api_key.clone(),
        model: model.model.clone(),
        dimension,
        batch_size,
        fastembed_cache_dir,
    })
}

pub(super) fn make_provider_from_default_model(
    model: Option<EmbeddingProviderConfig>,
    embedding_models_dir: PathBuf,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let model = model.ok_or_else(missing_default_embedding_config_error)?;
    make_provider_from_model(&model, embedding_models_dir)
}

fn make_provider(config: ResolvedEmbeddingConfig) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let backend = embedding_provider_capabilities(&config.backend).ok_or_else(|| {
        AppError::ConfigError(format!(
            "嵌入配置 {} 使用不支持的后端: {}",
            config.name, config.backend
        ))
    })?;
    let model = required_text(Some(&config.model), "模型名", &config.name)?;

    match backend.kind {
        EmbeddingBackendKind::Api => {
            let base_url = required_text(config.base_url.as_deref(), "API Base URL", &config.name)?;
            Ok(Arc::new(HttpEmbeddingProvider::new_with_batch(
                base_url,
                model,
                config.dimension,
                config.api_key,
                config.batch_size,
            )))
        }
        EmbeddingBackendKind::LocalHash => {
            Ok(Arc::new(LocalEmbeddingProvider::new(config.dimension)))
        }
        EmbeddingBackendKind::FastEmbed => make_fastembed_provider(
            &config.name,
            model,
            config.dimension,
            config.batch_size,
            config.fastembed_cache_dir,
        ),
    }
}

#[cfg(feature = "local-embedding")]
fn make_fastembed_provider(
    config_name: &str,
    model: String,
    dimension: usize,
    batch_size: usize,
    cache_dir: PathBuf,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let provider = FastEmbedProvider::try_new(&model, batch_size, cache_dir)?;
    if provider.dimension() != dimension {
        return Err(AppError::ConfigError(format!(
            "嵌入配置 {} 的向量维度为 {}，但 fastembed 模型 {} 实际输出 {} 维",
            config_name,
            dimension,
            provider.model_name(),
            provider.dimension()
        )));
    }
    Ok(Arc::new(provider))
}

#[cfg(not(feature = "local-embedding"))]
fn make_fastembed_provider(
    config_name: &str,
    _model: String,
    _dimension: usize,
    _batch_size: usize,
    _cache_dir: PathBuf,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    Err(AppError::ConfigError(format!(
        "嵌入配置 {config_name} 使用 fastembed，但当前 runtime 未启用 local-embedding；请使用 embedding sidecar 或启用该 feature"
    )))
}

fn missing_default_embedding_config_error() -> AppError {
    AppError::ConfigError("请先设置默认 Embedding 配置".to_string())
}
