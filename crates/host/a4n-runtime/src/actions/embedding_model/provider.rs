use super::LocalEmbeddingConfigInput;
#[cfg(feature = "local-embedding")]
use super::host_adapter::required_text;
#[cfg(feature = "local-embedding")]
use a4n_domain::embedding::{EmbeddingBackendKind, embedding_provider_capabilities};
#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::FastEmbedProvider;
use a4n_embedding_provider_contract::EmbeddingProvider;
use a4n_error::AppError;
#[cfg(feature = "local-embedding")]
use std::path::PathBuf;
use std::sync::Arc;

use super::runtime::LocalEmbeddingProviderRuntime;

pub(super) async fn cached_local_embedding_provider(
    runtime: &LocalEmbeddingProviderRuntime,
    input: &LocalEmbeddingConfigInput,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    #[cfg(feature = "local-embedding")]
    {
        let (model, dimension, batch_size, cache_dir) = resolve_fastembed_config(runtime, input)?;
        let key = (model.clone(), dimension, batch_size, cache_dir.clone());
        if let Some(provider) = runtime.providers.lock().await.get(&key).cloned() {
            return Ok(provider);
        }

        let provider = FastEmbedProvider::try_new(&model, batch_size, cache_dir)?;
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
) -> Result<(String, usize, usize, PathBuf), AppError> {
    let backend = input.backend.trim();
    if backend.is_empty() {
        return Err(AppError::InvalidArgument("嵌入后端不能为空".to_string()));
    }
    let capabilities = embedding_provider_capabilities(backend)
        .ok_or_else(|| AppError::InvalidArgument(format!("不支持的嵌入后端: {backend}")))?;
    match capabilities.kind {
        EmbeddingBackendKind::FastEmbed => {}
        EmbeddingBackendKind::LocalHash => {
            return Err(AppError::InvalidArgument(
                "远程桌面本机 Embedding 模式不支持 local_hash，请选择 fastembed 并下载本机模型"
                    .to_string(),
            ));
        }
        EmbeddingBackendKind::Api => {
            return Err(AppError::InvalidArgument(
                "远程桌面本机 Embedding 模式不支持 API 后端，请选择 fastembed 并下载本机模型"
                    .to_string(),
            ));
        }
    }
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
    Ok((model, dimension, batch_size, cache_dir))
}

#[cfg(not(feature = "local-embedding"))]
pub(super) fn local_embedding_disabled_error() -> AppError {
    AppError::ConfigError(
        "本地 fastembed 能力未启用：请使用 embedding sidecar，或用 local-embedding feature 构建 runtime"
            .to_string(),
    )
}
