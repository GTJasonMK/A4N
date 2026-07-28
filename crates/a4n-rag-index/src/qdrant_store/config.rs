use a4n_core::error::AppError;
use std::env;

/// Qdrant 连接与 collection 配置。
#[derive(Debug, Clone)]
pub struct QdrantStoreConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub collection: String,
}

impl QdrantStoreConfig {
    /// 从环境变量加载（`A4N_QDRANT_URL` 必填）。
    pub fn from_env() -> Result<Self, AppError> {
        let url = env::var("A4N_QDRANT_URL")
            .map_err(|_| {
                AppError::ConfigError("缺少 A4N_QDRANT_URL（Qdrant gRPC/HTTP 地址）".into())
            })?
            .trim()
            .to_string();
        if url.is_empty() {
            return Err(AppError::ConfigError("A4N_QDRANT_URL 不能为空".into()));
        }
        let api_key = env::var("A4N_QDRANT_API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let collection = env::var("A4N_QDRANT_COLLECTION")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "a4n_rag".to_string());
        Ok(Self {
            url,
            api_key,
            collection,
        })
    }
}
