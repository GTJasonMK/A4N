use a4n_domain::ProviderError;
use a4n_error::AppError;

pub(crate) fn from_app_error(error: AppError) -> ProviderError {
    match error {
        AppError::InvalidArgument(message) => ProviderError::InvalidArgument(message),
        AppError::NotFound(message) => ProviderError::NotFound(message),
        AppError::LlmError(message) => ProviderError::Provider(message),
        AppError::VectorStoreError(message) => ProviderError::VectorStore(message),
        AppError::EmbeddingError(message) => ProviderError::Embedding(message),
        AppError::Cancelled => ProviderError::Cancelled,
        AppError::Timeout(message) => ProviderError::Timeout(message),
        AppError::ConfigError(message) => ProviderError::Configuration(message),
        AppError::Internal(error) => ProviderError::Internal(format!("{error:#}")),
        AppError::Database(error) => ProviderError::Internal(format!("数据库错误：{error}")),
        AppError::Forbidden(message) => ProviderError::Internal(format!("权限不足：{message}")),
        AppError::Unauthorized(message) => ProviderError::Internal(format!("未认证：{message}")),
        AppError::InvalidState { current, operation } => ProviderError::Internal(format!(
            "状态无效：当前状态 {current}，不允许操作 {operation}"
        )),
        AppError::Io(error) => ProviderError::Internal(format!("IO 错误：{error}")),
        AppError::Serialization(error) => ProviderError::Internal(format!("序列化错误：{error}")),
    }
}
