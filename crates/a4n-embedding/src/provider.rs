//! EmbeddingProvider trait：文本嵌入模型抽象
//!
//! 支持多后端：远程 API（OpenAI/Ollama）、fastembed 本地模型、本地哈希。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use a4n_core::error::AppError;

pub const EMBEDDING_CONNECTIVITY_TEST_TEXT: &str = "test";

/// 嵌入后端类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingBackend {
    /// OpenAI/Ollama 兼容 API
    Api,
    /// 动态配置包装器
    Dynamic,
    /// 本地确定性哈希（显式开发/离线后端，不是真实语义模型）
    LocalHash,
    /// fastembed 本地 ONNX 嵌入模型
    FastEmbed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub message: String,
}

/// EmbeddingProvider trait
///
/// 文本嵌入计算抽象。支持批量嵌入以提升吞吐。
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// 批量嵌入文本
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError>;

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

    /// 测试连接可用性，并校验单条测试输入的返回数量与向量维度。
    async fn test_connection(&self) -> Result<(), AppError> {
        let expected_dimension = self.dimension();
        if expected_dimension == 0 {
            return Err(AppError::ConfigError(
                "嵌入连接测试配置维度必须为正整数".to_string(),
            ));
        }
        let embeddings = self
            .embed(vec![EMBEDDING_CONNECTIVITY_TEST_TEXT.to_string()])
            .await?;
        let [embedding] = embeddings.as_slice() else {
            return Err(AppError::EmbeddingError(format!(
                "嵌入连接测试返回 {} 个向量，期望 1 个",
                embeddings.len()
            )));
        };
        if embedding.len() != expected_dimension {
            return Err(AppError::EmbeddingError(format!(
                "嵌入连接测试向量维度不匹配：返回 {}，期望 {expected_dimension}",
                embedding.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use async_trait::async_trait;

    struct FixedEmbeddingProvider {
        dimension: usize,
        embeddings: Vec<Vec<f32>>,
    }

    #[async_trait]
    impl EmbeddingProvider for FixedEmbeddingProvider {
        async fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
            Ok(self.embeddings.clone())
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        fn model_name(&self) -> &str {
            "fixed"
        }

        fn backend(&self) -> EmbeddingBackend {
            EmbeddingBackend::LocalHash
        }
    }

    #[test]
    fn embedding_identity_rejects_unknown_fields() {
        let error = serde_json::from_value::<EmbeddingModelIdentity>(serde_json::json!({
            "backend": "FastEmbed",
            "model": "model",
            "dimension": 384,
            "legacy_backend": "local"
        }))
        .expect_err("embedding identity must reject unknown fields");
        assert!(error.to_string().contains("legacy_backend"));
    }

    #[tokio::test]
    async fn connection_test_requires_one_vector_with_configured_dimension() {
        FixedEmbeddingProvider {
            dimension: 2,
            embeddings: vec![vec![0.0, 1.0]],
        }
        .test_connection()
        .await
        .expect("matching connection response");

        for provider in [
            FixedEmbeddingProvider {
                dimension: 2,
                embeddings: Vec::new(),
            },
            FixedEmbeddingProvider {
                dimension: 2,
                embeddings: vec![vec![0.0]],
            },
            FixedEmbeddingProvider {
                dimension: 0,
                embeddings: vec![Vec::new()],
            },
        ] {
            assert!(provider.test_connection().await.is_err());
        }
    }
}
