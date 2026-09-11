use a4n_domain::ProviderError as AppError;
use a4n_rag_core::DEFAULT_QDRANT_COLLECTION;

fn optional_non_empty_env_string(key: &str) -> Result<Option<String>, AppError> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(AppError::Configuration(format!("{key} 不能为空")))
            } else {
                Ok(Some(value))
            }
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(AppError::Configuration(format!("{key} 无法读取: {error}"))),
    }
}

fn required_non_empty_env_string(key: &str) -> Result<String, AppError> {
    optional_non_empty_env_string(key)?
        .ok_or_else(|| AppError::Configuration(format!("{key} 未设置")))
}

/// Qdrant 连接与 collection 配置。
pub struct QdrantStoreConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub collection: String,
}

impl QdrantStoreConfig {
    /// 从环境变量加载（`A4N_QDRANT_URL` 必填）。
    pub(super) fn from_env() -> Result<Self, AppError> {
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
