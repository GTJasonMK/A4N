//! 本地嵌入后端
//!
//! 基于文本哈希的**确定性**嵌入向量（不是随机）：相同文本产生相同向量，
//! RAG 去重、相似度检索都能得到至少符合"相同文本得相同向量"语义的结果。
//!
//! 完整 transformer 质量请改用：
//!   - `HttpEmbeddingProvider`：调 OpenAI 兼容 `/v1/embeddings` 端点
//!   - `FastEmbedProvider`：使用当前支持的本地 ONNX 嵌入模型

use a4n_domain::ProviderError as AppError;
use a4n_embedding_provider_contract::{EmbeddingBackend, EmbeddingProvider};
use async_trait::async_trait;
use sha2::{Digest, Sha256};

/// 本地嵌入提供者
///
/// 使用 SHA-256 把文本哈希到 `dimension` 维向量（确定性）。
/// 同一文本始终产生同一向量；不同文本产生不同向量。
/// 输出 L2 归一化，可直接用于余弦相似度。
pub struct LocalEmbeddingProvider {
    /// 向量维度
    dimension: usize,
}

impl LocalEmbeddingProvider {
    /// 创建新的本地嵌入提供者
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    /// 把文本哈希成 `dimension` 维归一化向量
    fn hash_embedding(&self, text: &str) -> Vec<f32> {
        // 用滚动 SHA-256 生成足够多字节填满 dimension*4（每 f32 取 4 bytes）
        let mut bytes: Vec<u8> = Vec::with_capacity(self.dimension * 4);
        let mut counter: u32 = 0;
        while bytes.len() < self.dimension * 4 {
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            hasher.update(counter.to_le_bytes());
            bytes.extend_from_slice(&hasher.finalize());
            counter += 1;
        }

        // 每 4 字节解析为 i32，再映射到 [-1, 1]
        let mut vec: Vec<f32> = (0..self.dimension)
            .map(|i| {
                let off = i * 4;
                let raw = i32::from_le_bytes([
                    bytes[off],
                    bytes[off + 1],
                    bytes[off + 2],
                    bytes[off + 3],
                ]);
                (raw as f32) / (i32::MAX as f32)
            })
            .collect();

        // L2 归一化
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vec {
                *x /= norm;
            }
        }
        vec
    }
}

#[async_trait]
impl EmbeddingProvider for LocalEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(texts.iter().map(|t| self.hash_embedding(t)).collect())
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        "local-sha256-hash"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_embedding_dimension_and_norm() {
        let provider = LocalEmbeddingProvider::new(384);
        let embeddings = provider
            .embed(vec!["测试文本".to_string()])
            .await
            .expect("嵌入计算失败");
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].len(), 384);
        // L2 归一化
        let norm: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
        provider.test_connection().await.expect("本地嵌入连接测试");
    }

    #[tokio::test]
    async fn test_determinism_same_text_same_vector() {
        let provider = LocalEmbeddingProvider::new(128);
        let a = provider.embed(vec!["hello".to_string()]).await.unwrap();
        let b = provider.embed(vec!["hello".to_string()]).await.unwrap();
        assert_eq!(a, b, "相同文本必须产生相同向量");
    }

    #[tokio::test]
    async fn test_distinct_text_distinct_vector() {
        let provider = LocalEmbeddingProvider::new(128);
        let a = provider.embed(vec!["hello".to_string()]).await.unwrap();
        let b = provider.embed(vec!["world".to_string()]).await.unwrap();
        assert_ne!(a, b, "不同文本应当产生不同向量");
    }

    #[tokio::test]
    async fn test_local_embedding_batch() {
        let provider = LocalEmbeddingProvider::new(384);
        let texts: Vec<String> = (0..5).map(|i| format!("文本 {}", i)).collect();
        let embeddings = provider.embed(texts.clone()).await.expect("批量嵌入失败");
        assert_eq!(embeddings.len(), 5);
        for emb in &embeddings {
            assert_eq!(emb.len(), 384);
        }
    }
}
