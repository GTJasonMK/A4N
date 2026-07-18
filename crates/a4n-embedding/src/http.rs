//! HTTP 嵌入 Provider：通过 OpenAI 兼容 API 计算文本嵌入向量
//!
//! 支持 OpenAI、Ollama 等所有兼容 `/embeddings` 端点的服务。

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::provider::{EmbeddingBackend, EmbeddingProvider};
use a4n_core::config::app_config::EmbeddingConfig;
use a4n_core::error::AppError;

fn internal_error(message: impl Into<String>) -> AppError {
    AppError::Internal(anyhow::anyhow!(message.into()))
}

async fn response_text(resp: reqwest::Response, label: &str) -> Result<String, AppError> {
    resp.text()
        .await
        .map_err(|e| internal_error(format!("{label}响应读取失败: {e}")))
}

fn required_array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>, AppError> {
    value
        .as_array()
        .ok_or_else(|| internal_error(format!("嵌入响应缺少数组字段 {field}")))
}

fn required_f32(value: &Value, field: &str) -> Result<f32, AppError> {
    value
        .as_f64()
        .map(|number| number as f32)
        .ok_or_else(|| internal_error(format!("嵌入响应字段 {field} 不是数字")))
}

/// HTTP 嵌入 Provider
///
/// 通过 HTTP 调用 OpenAI 兼容 API 的 `/embeddings` 端点。
pub struct HttpEmbeddingProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    dim: usize,
    batch: usize,
}

impl HttpEmbeddingProvider {
    /// 从 EmbeddingConfig 创建 Provider
    pub fn from_config(config: &EmbeddingConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("创建 HTTP 客户端失败");

        let base_url = config.base_url.trim_end_matches('/').to_string();

        Self {
            client,
            base_url,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            dim: config.dimension,
            batch: config.batch_size,
        }
    }

    /// 手动创建
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        dimension: usize,
        api_key: Option<String>,
    ) -> Self {
        Self::new_with_batch(base_url, model, dimension, api_key, 100)
    }

    /// 手动创建，并显式指定批量大小
    pub fn new_with_batch(
        base_url: impl Into<String>,
        model: impl Into<String>,
        dimension: usize,
        api_key: Option<String>,
        batch_size: usize,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("创建 HTTP 客户端失败");

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            model: model.into(),
            dim: dimension,
            batch: batch_size,
        }
    }

    /// 单批请求
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, AppError> {
        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let mut req = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .json(&body);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("嵌入 API 请求失败: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = response_text(resp, "嵌入 API").await?;
            return Err(internal_error(format!(
                "嵌入 API 返回错误 {status}: {body_text}"
            )));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| internal_error(format!("解析嵌入响应失败: {e}")))?;

        let data = required_array(&json["data"], "data")?;
        if data.len() != texts.len() {
            return Err(internal_error(format!(
                "嵌入响应数量不匹配: 请求 {} 条，返回 {} 条",
                texts.len(),
                data.len()
            )));
        }

        let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(data.len());
        for (item_index, item) in data.iter().enumerate() {
            let embedding =
                required_array(&item["embedding"], &format!("data[{item_index}].embedding"))?;
            if embedding.len() != self.dim {
                return Err(internal_error(format!(
                    "嵌入响应维度不匹配: data[{item_index}].embedding 长度为 {}，期望 {}",
                    embedding.len(),
                    self.dim
                )));
            }
            let vec = embedding
                .iter()
                .enumerate()
                .map(|(dim_index, value)| {
                    required_f32(value, &format!("data[{item_index}].embedding[{dim_index}]"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            embeddings.push(vec);
        }

        Ok(embeddings)
    }
}

#[async_trait]
impl EmbeddingProvider for HttpEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        if self.batch == 0 {
            return Err(internal_error("嵌入批量大小不能为 0"));
        }

        // 分批请求
        let mut all_embeddings = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(self.batch) {
            let batch: Vec<String> = chunk.to_vec();
            let mut embeddings = self.embed_batch(&batch).await?;
            all_embeddings.append(&mut embeddings);
        }

        Ok(all_embeddings)
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::Api
    }

    fn batch_size(&self) -> usize {
        self.batch
    }
}

impl std::fmt::Debug for HttpEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpEmbeddingProvider")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("dimension", &self.dim)
            .finish()
    }
}
