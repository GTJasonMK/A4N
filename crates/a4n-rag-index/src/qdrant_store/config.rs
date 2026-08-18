use a4n_core::config::env::{optional_non_empty_env_string, required_non_empty_env_string};
use a4n_core::error::AppError;
use a4n_rag_core::DEFAULT_QDRANT_COLLECTION;

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
        let url = required_non_empty_env_string("A4N_QDRANT_URL")?;
        let api_key = optional_non_empty_env_string("A4N_QDRANT_API_KEY")?;
        let collection = optional_non_empty_env_string("A4N_QDRANT_COLLECTION")?
            .unwrap_or_else(|| DEFAULT_QDRANT_COLLECTION.to_string());
        Ok(Self {
            url,
            api_key,
            collection,
        })
    }
}
