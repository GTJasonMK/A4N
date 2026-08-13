use super::LocalEmbeddingConfigInput;
#[cfg(feature = "local-embedding")]
use super::host_adapter::required_text;
#[cfg(feature = "local-embedding")]
use a4n_core::config::{EmbeddingBackendKind, embedding_provider_capabilities};
use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingProvider;
#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::FastEmbedProvider;
use std::{path::PathBuf, sync::Arc};

use super::runtime::LocalEmbeddingProviderRuntime;

pub(super) fn fastembed_cache_dir_from_config(
    config: &a4n_core::config::app_config::AppConfig,
) -> PathBuf {
    crate::dynamic_embedding::effective_fastembed_cache_dir(
        crate::dynamic_embedding::default_fastembed_cache_dir(config),
    )
}

pub(super) async fn cached_local_embedding_provider(
    runtime: &LocalEmbeddingProviderRuntime,
    input: &LocalEmbeddingConfigInput,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    #[cfg(feature = "local-embedding")]
    {
        let (backend, model, dimension, batch_size, cache_dir) =
            resolve_fastembed_config(runtime, input)?;
        let key = local_provider_cache_key(&backend, &model, dimension, batch_size, &cache_dir);
        if let Some(provider) = runtime.providers.lock().await.get(&key).cloned() {
            return Ok(provider);
        }

        let provider =
            FastEmbedProvider::try_new(model, batch_size, cache_dir).map_err(AppError::from)?;
        if provider.dimension() != dimension {
            return Err(AppError::ConfigError(format!(
                "嵌入配置维度为 {dimension}，但 fastembed 模型 {} 实际输出 {} 维",
                provider.model_name(),
                provider.dimension()
            )));
        }
        let provider: Arc<dyn EmbeddingProvider> = Arc::new(provider);
        runtime.providers.lock().await.insert(key, provider.clone());
        Ok(provider)
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = (runtime, input);
        Err(local_embedding_disabled_error())
    }
}

#[cfg(feature = "local-embedding")]
pub(super) fn resolve_fastembed_config(
    runtime: &LocalEmbeddingProviderRuntime,
    input: &LocalEmbeddingConfigInput,
) -> Result<(String, String, usize, usize, PathBuf), AppError> {
    let backend = normalized_fastembed_backend_name(&input.backend)?;
    let model = required_text(&input.model, "模型名")?;
    let dimension = input.dimension;
    if dimension == 0 {
        return Err(AppError::InvalidArgument("向量维度必须大于 0".to_string()));
    }
    let batch_size = input.batch_size;
    if batch_size == 0 {
        return Err(AppError::InvalidArgument("批量大小必须大于 0".to_string()));
    }
    let cache_dir = crate::dynamic_embedding::resolve_fastembed_cache_dir(
        input.local_model_path.as_deref(),
        runtime.fastembed_cache_dir(),
    )?;
    Ok((backend, model, dimension, batch_size, cache_dir))
}

#[cfg(feature = "local-embedding")]
fn normalized_fastembed_backend_name(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument("嵌入后端不能为空".to_string()));
    }
    let capabilities = embedding_provider_capabilities(value)
        .ok_or_else(|| AppError::InvalidArgument(format!("不支持的嵌入后端: {value}")))?;
    match capabilities.kind {
        EmbeddingBackendKind::FastEmbed => Ok(EmbeddingBackendKind::FastEmbed
            .identity_backend_name()
            .to_string()),
        EmbeddingBackendKind::LocalHash => Err(AppError::InvalidArgument(
            "远程桌面本机 Embedding 模式不支持 local_hash，请选择 fastembed 并下载本机模型"
                .to_string(),
        )),
        EmbeddingBackendKind::Api => Err(AppError::InvalidArgument(
            "远程桌面本机 Embedding 模式不支持 API 后端，请选择 fastembed 并下载本机模型"
                .to_string(),
        )),
    }
}

#[cfg(feature = "local-embedding")]
fn local_provider_cache_key(
    backend: &str,
    model: &str,
    dimension: usize,
    batch_size: usize,
    cache_dir: &std::path::Path,
) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        backend,
        model,
        dimension,
        batch_size,
        cache_dir.to_string_lossy()
    )
}

#[cfg(not(feature = "local-embedding"))]
pub(super) fn local_embedding_disabled_error() -> AppError {
    AppError::ConfigError(
        "本地 fastembed 能力未启用：请使用 embedding sidecar，或用 local-embedding feature 构建 runtime"
            .to_string(),
    )
}
