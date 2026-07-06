//! EmbeddingProvider trait：文本嵌入模型抽象
//!
//! 支持多后端：远程 API（OpenAI/Ollama）、fastembed 本地模型、本地哈希。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use a4n_core::error::AppError;

/// 嵌入后端类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingBackend {
    /// OpenAI/Ollama 兼容 API
    Api,
    /// 动态配置包装器
    Dynamic,
    /// 本地确定性哈希（开发/离线兜底，不是真实语义模型）
    LocalHash,
    /// fastembed 本地 ONNX 嵌入模型
    FastEmbed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingModelIdentity {
    pub backend: String,
    pub model: String,
    pub dimension: usize,
}

impl EmbeddingModelIdentity {
    pub fn new(backend: EmbeddingBackend, model: impl AsRef<str>, dimension: usize) -> Self {
        Self {
            backend: format!("{backend:?}"),
            model: model.as_ref().trim().to_string(),
            dimension,
        }
    }
}

/// 模型下载进度
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
    pub message: String,
}

/// EmbeddingProvider trait
///
/// 文本嵌入计算抽象。支持批量嵌入以提升吞吐。
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// 批量嵌入文本
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError>;

    /// 单个文本嵌入
    async fn embed_one(&self, text: String) -> Result<Vec<f32>, AppError> {
        let mut results = self.embed(vec![text]).await?;
        results
            .pop()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("嵌入返回为空")))
    }

    /// 嵌入向量维度
    fn dimension(&self) -> usize;

    /// 模型名称
    fn model_name(&self) -> &str;

    /// 后端类型
    fn backend(&self) -> EmbeddingBackend;

    /// 批量大小限制
    fn batch_size(&self) -> usize {
        100
    }

    /// 当前实际用于嵌入的模型身份，用于判断已有向量是否需要重建。
    async fn model_identity(&self) -> Result<EmbeddingModelIdentity, AppError> {
        Ok(EmbeddingModelIdentity::new(
            self.backend(),
            self.model_name(),
            self.dimension(),
        ))
    }

    /// 批量嵌入并返回同一次调用使用的模型身份。
    async fn embed_with_identity(
        &self,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), AppError> {
        let identity = self.model_identity().await?;
        let embeddings = self.embed(texts).await?;
        Ok((embeddings, identity))
    }

    /// 测试连接可用性
    async fn test_connection(&self) -> Result<bool, AppError> {
        self.embed_one("测试".to_string()).await.map(|_| true)
    }

    /// 下载模型（支持断点续传和进度回调）
    ///
    /// 默认实现为空（API 后端不需要下载）。
    async fn download_model(
        &self,
        _progress_tx: tokio::sync::mpsc::UnboundedSender<DownloadProgress>,
    ) -> Result<(), AppError> {
        Ok(())
    }
}
