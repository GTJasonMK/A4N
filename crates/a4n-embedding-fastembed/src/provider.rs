use std::path::PathBuf;

use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingProvider};
use async_trait::async_trait;
use fastembed::{EmbeddingModel, ModelInfo, TextEmbedding, TextInitOptions};
use tokio::sync::Mutex;

use crate::cache::{model_cache_path, snapshot_with_required_files};
use crate::models::resolve_model_info;
use crate::proxy::ScopedHfHubEnv;

pub struct FastEmbedProvider {
    model_name: String,
    model_info: ModelInfo<EmbeddingModel>,
    batch_size: usize,
    cache_dir: PathBuf,
    inner: Mutex<Option<TextEmbedding>>,
}

impl FastEmbedProvider {
    pub fn try_new(
        model_name: impl AsRef<str>,
        batch_size: usize,
        cache_dir: PathBuf,
    ) -> Result<Self, AppError> {
        let resolved = resolve_model_info(model_name.as_ref())?;
        if batch_size == 0 {
            return Err(AppError::ConfigError(
                "fastembed 嵌入批量大小必须为正整数".to_string(),
            ));
        }

        Ok(Self {
            model_name: resolved.name,
            model_info: resolved.info,
            batch_size,
            cache_dir,
            inner: Mutex::new(None),
        })
    }

    fn ensure_downloaded(&self) -> Result<(), AppError> {
        let cache_path = model_cache_path(&self.model_info, &self.cache_dir);
        if snapshot_with_required_files(&self.model_info, &cache_path)?.is_some() {
            return Ok(());
        }
        Err(AppError::ConfigError(format!(
            "本地嵌入模型 {} 尚未下载，请先在设置页下载后再使用",
            self.model_name
        )))
    }

    async fn take_or_init_model(&self) -> Result<TextEmbedding, AppError> {
        let mut guard = self.inner.lock().await;
        if let Some(model) = guard.take() {
            return Ok(model);
        }

        let model = self.model_info.model.clone();
        let cache_dir = self.cache_dir.clone();
        drop(guard);

        tokio::task::spawn_blocking(move || build_text_embedding(model, cache_dir))
            .await
            .map_err(|err| {
                AppError::EmbeddingError(format!("初始化 fastembed 线程失败: {err}"))
            })?
            .map_err(|err| {
                AppError::EmbeddingError(format!(
                    "初始化 fastembed 模型失败：本地模型缓存可能不完整，请在设置页删除缓存后重新下载；底层错误：{err}"
                ))
            })
    }
}

#[async_trait]
impl EmbeddingProvider for FastEmbedProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        self.ensure_downloaded()?;

        let mut model = self.take_or_init_model().await?;
        let batch_size = self.batch_size;
        let (model, embeddings) = tokio::task::spawn_blocking(move || {
            let embeddings = model
                .embed(texts, Some(batch_size))
                .map_err(|err| AppError::EmbeddingError(format!("fastembed 嵌入失败: {err}")));
            (model, embeddings)
        })
        .await
        .map_err(|err| AppError::EmbeddingError(format!("执行 fastembed 线程失败: {err}")))?;
        *self.inner.lock().await = Some(model);
        embeddings
    }

    fn dimension(&self) -> usize {
        self.model_info.dim
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::FastEmbed
    }

    fn batch_size(&self) -> usize {
        self.batch_size
    }
}

pub(super) fn build_text_embedding(
    model: EmbeddingModel,
    cache_dir: PathBuf,
) -> Result<TextEmbedding, String> {
    std::fs::create_dir_all(&cache_dir).map_err(|err| format!("创建模型缓存目录失败：{err}"))?;
    let _hf_env = ScopedHfHubEnv::set(&cache_dir)?;
    TextEmbedding::try_new(
        TextInitOptions::new(model)
            .with_cache_dir(cache_dir)
            .with_show_download_progress(false),
    )
    .map_err(|err| format_error_chain(&err))
}

fn format_error_chain(err: &anyhow::Error) -> String {
    err.chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("；")
}
