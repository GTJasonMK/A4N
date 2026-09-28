use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex, OnceLock};

use a4n_domain::ProviderError as AppError;
use a4n_embedding_provider_contract::{EmbeddingBackend, EmbeddingProvider};
use async_trait::async_trait;
use fastembed::{EmbeddingModel, ModelInfo, TextEmbedding, TextInitOptions};
use tokio::sync::{Mutex, OwnedMutexGuard};

use crate::cache::{model_cache_path, snapshot_with_required_files};
use crate::models::resolve_model_info;
use crate::proxy::ScopedHfHubEnv;

type ModelSlot = Arc<Mutex<Option<TextEmbedding>>>;

struct RetainedModel {
    model_name: String,
    cache_dir: PathBuf,
    slot: ModelSlot,
}

// Keep one otherwise-unused model alive for stateless RPC providers. Live
// providers still own their slots, including when another model is selected.
// ponytail: one retained configuration; add a bounded pool only if mixed-model load justifies its RAM.
static RETAINED_MODEL: StdMutex<Option<RetainedModel>> = StdMutex::new(None);

fn clear_retained_model() {
    let previous = RETAINED_MODEL
        .lock()
        .expect("model retention lock poisoned")
        .take();
    drop(previous);
}

pub(super) struct ModelFilesMutation;

impl ModelFilesMutation {
    pub(super) fn new() -> Self {
        clear_retained_model();
        Self
    }
}

impl Drop for ModelFilesMutation {
    fn drop(&mut self) {
        // Also invalidate models opened while a download/delete was running,
        // including when its future is cancelled or the mutation fails.
        clear_retained_model();
    }
}

pub struct FastEmbedProvider {
    model_name: String,
    model_info: ModelInfo<EmbeddingModel>,
    batch_size: usize,
    cache_dir: PathBuf,
    inner: OnceLock<ModelSlot>,
}

impl FastEmbedProvider {
    pub fn try_new(
        model_name: impl AsRef<str>,
        batch_size: usize,
        cache_dir: PathBuf,
    ) -> Result<Self, AppError> {
        let resolved = resolve_model_info(model_name.as_ref())?;
        if batch_size == 0 {
            return Err(AppError::Configuration(
                "fastembed 嵌入批量大小必须为正整数".to_string(),
            ));
        }

        Ok(Self {
            model_name: resolved.name,
            model_info: resolved.info,
            batch_size,
            cache_dir,
            inner: OnceLock::new(),
        })
    }

    fn ensure_downloaded(&self) -> Result<(), AppError> {
        let cache_path = model_cache_path(&self.model_info, &self.cache_dir);
        if snapshot_with_required_files(&self.model_info, &cache_path)?.is_some() {
            return Ok(());
        }
        Err(AppError::Configuration(format!(
            "本地嵌入模型 {} 尚未下载，请先在设置页下载后再使用",
            self.model_name
        )))
    }

    fn model_slot(&self) -> ModelSlot {
        self.inner
            .get_or_init(|| {
                let mut retained = RETAINED_MODEL
                    .lock()
                    .expect("model retention lock poisoned");
                if let Some(current) = retained.as_ref()
                    && current.model_name == self.model_name
                    && current.cache_dir == self.cache_dir
                {
                    return current.slot.clone();
                }
                let slot = Arc::new(Mutex::new(None));
                let previous = retained.replace(RetainedModel {
                    model_name: self.model_name.clone(),
                    cache_dir: self.cache_dir.clone(),
                    slot: slot.clone(),
                });
                drop(retained);
                // Releasing a large ONNX session must not hold the registry lock.
                drop(previous);
                slot
            })
            .clone()
    }

    async fn model_guard(&self) -> Result<OwnedMutexGuard<Option<TextEmbedding>>, AppError> {
        let mut guard = self.model_slot().lock_owned().await;
        if guard.is_some() {
            return Ok(guard);
        }

        let model = self.model_info.model.clone();
        let cache_dir = self.cache_dir.clone();

        tokio::task::spawn_blocking(move || -> Result<_, String> {
            *guard = Some(build_text_embedding(model, cache_dir)?);
            Ok(guard)
        })
            .await
            .map_err(|err| {
                AppError::Embedding(format!("初始化 fastembed 线程失败: {err}"))
            })?
            .map_err(|err| {
                AppError::Embedding(format!(
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

        let mut guard = self.model_guard().await?;
        let batch_size = self.batch_size;
        tokio::task::spawn_blocking(move || {
            // The blocking operation owns the lock, so dropping the request
            // cannot let another request initialize a second copy mid-inference.
            let mut model = guard.take().expect("initialized fastembed model");
            let embeddings = model
                .embed(texts, Some(batch_size))
                .map_err(|err| AppError::Embedding(format!("fastembed 嵌入失败: {err}")));
            *guard = Some(model);
            embeddings
        })
        .await
        .map_err(|err| AppError::Embedding(format!("执行 fastembed 线程失败: {err}")))?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_provider_ownership_survives_registry_invalidation() {
        let directory = tempfile::tempdir().unwrap();
        let provider =
            FastEmbedProvider::try_new("BGESmallZHV15", 32, directory.path().to_path_buf())
                .unwrap();
        let original = provider.model_slot();
        let mutation = ModelFilesMutation::new();
        let during = FastEmbedProvider::try_new("BGESmallZHV15", 1, directory.path().to_path_buf())
            .unwrap()
            .model_slot();
        assert!(!Arc::ptr_eq(&original, &during));
        drop(mutation);
        let after = FastEmbedProvider::try_new("BGESmallZHV15", 32, directory.path().to_path_buf())
            .unwrap()
            .model_slot();
        assert!(!Arc::ptr_eq(&during, &after));
        assert!(Arc::ptr_eq(&provider.model_slot(), &original));
        clear_retained_model();
    }
}
