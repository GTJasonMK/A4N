use thiserror::Error;

/// Errors produced by provider implementations before they cross the Host
/// boundary.  The type intentionally carries no database or runtime details.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("参数无效：{0}")]
    InvalidArgument(String),

    #[error("配置错误：{0}")]
    Configuration(String),

    #[error("供应商错误：{0}")]
    Provider(String),

    #[error("嵌入计算失败：{0}")]
    Embedding(String),

    #[error("向量检索失败：{0}")]
    VectorStore(String),

    #[error("操作超时：{0}")]
    Timeout(String),

    #[error("操作已取消")]
    Cancelled,

    #[error("内部错误：{0}")]
    Internal(String),

    #[error("未找到：{0}")]
    NotFound(String),
}

impl From<std::io::Error> for ProviderError {
    fn from(error: std::io::Error) -> Self {
        Self::Internal(error.to_string())
    }
}
