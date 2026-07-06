//! 动态 Embedding Provider：每次调用时解析当前默认嵌入配置。

use std::path::PathBuf;
use std::sync::Arc;

use a4n_core::config::app_config;
use a4n_core::config::app_config::AppConfig;
use a4n_core::error::AppError;
use a4n_core::models::ai::embedding_config;
use a4n_engine::embedding::fastembed::FastEmbedProvider;
use a4n_engine::embedding::http::HttpEmbeddingProvider;
use a4n_engine::embedding::local::LocalEmbeddingProvider;
use a4n_engine::embedding::provider::{
    DownloadProgress, EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider,
};
use a4n_engine::queue::priority::{TaskPriority, TaskType};
use a4n_services::queue::PersistentTaskQueue;
use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tokio::sync::mpsc::UnboundedSender;

use crate::runtime_config;

pub struct DynamicEmbeddingProvider {
    db: Arc<DatabaseConnection>,
    task_queue: Arc<PersistentTaskQueue>,
    metadata_config: app_config::EmbeddingConfig,
    fastembed_cache_dir: PathBuf,
}

impl DynamicEmbeddingProvider {
    pub fn new(
        db: Arc<DatabaseConnection>,
        task_queue: Arc<PersistentTaskQueue>,
        metadata_config: app_config::EmbeddingConfig,
        fastembed_cache_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            task_queue,
            metadata_config,
            fastembed_cache_dir,
        }
    }

    async fn resolve_default(&self) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
        if let Some(model) = runtime_config::current_embedding_model() {
            return make_provider_from_model(&model, self.fastembed_cache_dir.clone());
        }

        let model = embedding_config::Entity::find()
            .filter(embedding_config::Column::IsDefault.eq(true))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        make_provider_from_default_model(model, self.fastembed_cache_dir.clone())
    }
}

pub fn default_fastembed_cache_dir(config: &AppConfig) -> PathBuf {
    config.cache_dir.join("embedding-models")
}

pub fn effective_fastembed_cache_dir(default_cache_dir: PathBuf) -> PathBuf {
    app_config::resolve_portable_path(default_cache_dir)
}

pub fn make_provider_from_model(
    model: &embedding_config::Model,
    default_fastembed_cache_dir: PathBuf,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let dimension = positive_usize(model.dimension, "向量维度", &model.name)?;
    let batch_size = positive_usize(model.batch_size, "批量大小", &model.name)?;
    let fastembed_cache_dir = resolve_fastembed_cache_dir(
        model.local_model_path.as_deref(),
        default_fastembed_cache_dir,
    );
    make_provider(ResolvedEmbeddingConfig {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        dimension,
        batch_size,
        fastembed_cache_dir,
    })
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

fn make_provider_from_default_model(
    model: Option<embedding_config::Model>,
    default_fastembed_cache_dir: PathBuf,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let model = model.ok_or_else(missing_default_embedding_config_error)?;
    make_provider_from_model(&model, default_fastembed_cache_dir)
}

fn make_provider(config: ResolvedEmbeddingConfig) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let backend = SupportedEmbeddingBackend::parse(&config.backend).ok_or_else(|| {
        AppError::ConfigError(format!(
            "嵌入配置 {} 使用不支持的后端: {}",
            config.name, config.backend
        ))
    })?;
    let model = required_text(&config.model, "模型名", &config.name)?;

    match backend {
        SupportedEmbeddingBackend::Api => {
            let base_url = required_text(
                config.base_url.as_deref().unwrap_or(""),
                "API Base URL",
                &config.name,
            )?;
            Ok(Arc::new(HttpEmbeddingProvider::new_with_batch(
                normalize_api_base_url(base_url),
                model,
                config.dimension,
                config.api_key,
                config.batch_size,
            )))
        }
        SupportedEmbeddingBackend::LocalHash => {
            Ok(Arc::new(LocalEmbeddingProvider::new(config.dimension)))
        }
        SupportedEmbeddingBackend::FastEmbed => {
            let provider =
                FastEmbedProvider::try_new(&model, config.batch_size, config.fastembed_cache_dir)?;
            if provider.dimension() != config.dimension {
                return Err(AppError::ConfigError(format!(
                    "嵌入配置 {} 的向量维度为 {}，但 fastembed 模型 {} 实际输出 {} 维",
                    config.name,
                    config.dimension,
                    provider.model_name(),
                    provider.dimension()
                )));
            }
            Ok(Arc::new(provider))
        }
    }
}

fn resolve_fastembed_cache_dir(value: Option<&str>, default_cache_dir: PathBuf) -> PathBuf {
    let cache_dir = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(default_cache_dir);
    effective_fastembed_cache_dir(cache_dir)
}

fn positive_usize(value: i32, label: &str, name: &str) -> Result<usize, AppError> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::ConfigError(format!("嵌入配置 {name} 的{label}必须为正整数")))
}

fn required_text(value: &str, label: &str, name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::ConfigError(format!(
            "嵌入配置 {name} 缺少{label}"
        )))
    } else {
        Ok(value.to_string())
    }
}

fn normalize_api_base_url(base_url: String) -> String {
    let trimmed = base_url.trim_end_matches('/').to_string();
    let lower = trimmed.to_ascii_lowercase();
    let looks_like_ollama = lower.contains("localhost:11434")
        || lower.contains("127.0.0.1:11434")
        || lower.contains(":11434");
    if looks_like_ollama && !trimmed.ends_with("/v1") {
        return format!("{trimmed}/v1");
    }
    trimmed
}

fn missing_default_embedding_config_error() -> AppError {
    AppError::ConfigError("请先设置默认 Embedding 配置".to_string())
}

struct ResolvedEmbeddingConfig {
    name: String,
    backend: String,
    base_url: Option<String>,
    api_key: Option<String>,
    model: String,
    dimension: usize,
    batch_size: usize,
    fastembed_cache_dir: PathBuf,
}

enum SupportedEmbeddingBackend {
    Api,
    LocalHash,
    FastEmbed,
}

impl SupportedEmbeddingBackend {
    fn parse(value: &str) -> Option<Self> {
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

#[async_trait]
impl EmbeddingProvider for DynamicEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        let provider = self.resolve_default().await?;
        self.task_queue
            .run_tracked(
                TaskType::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move { provider.embed(texts).await },
            )
            .await
    }

    fn dimension(&self) -> usize {
        self.metadata_config.dimension
    }

    fn model_name(&self) -> &str {
        "dynamic-default-embedding"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::Dynamic
    }

    fn batch_size(&self) -> usize {
        self.metadata_config.batch_size
    }

    async fn model_identity(&self) -> Result<EmbeddingModelIdentity, AppError> {
        self.resolve_default().await?.model_identity().await
    }

    async fn embed_with_identity(
        &self,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), AppError> {
        let provider = self.resolve_default().await?;
        self.task_queue
            .run_tracked(
                TaskType::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move {
                    let identity = provider.model_identity().await?;
                    let embeddings = provider.embed(texts).await?;
                    Ok((embeddings, identity))
                },
            )
            .await
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        let provider = self.resolve_default().await?;
        self.task_queue
            .run_tracked(
                TaskType::EmbeddingCall,
                TaskPriority::High,
                None,
                || async move { provider.test_connection().await },
            )
            .await
    }

    async fn download_model(
        &self,
        progress_tx: UnboundedSender<DownloadProgress>,
    ) -> Result<(), AppError> {
        let provider = self.resolve_default().await?;
        self.task_queue
            .run_tracked(
                TaskType::EmbeddingDownload,
                TaskPriority::Low,
                None,
                || async move { provider.download_model(progress_tx).await },
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_embedding_requires_database_default_config() {
        let err = match make_provider_from_default_model(None, PathBuf::from("unused")) {
            Ok(_) => panic!("missing default embedding config must fail visibly"),
            Err(err) => err,
        };

        assert!(matches!(
            err,
            AppError::ConfigError(message) if message == "请先设置默认 Embedding 配置"
        ));
    }
}
