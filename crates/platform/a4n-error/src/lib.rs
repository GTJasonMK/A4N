//! Application error boundary shared by adapters and host-facing services.
//!
//! This crate owns error semantics without depending on the core model crate.
//! Database and host layers can convert this type at their outer boundary.

use thiserror::Error;

pub fn append_secondary_message(primary: String, secondary: Option<String>) -> String {
    match secondary.filter(|message| !message.trim().is_empty()) {
        Some(secondary) => format!("{primary}；{secondary}"),
        None => primary,
    }
}

pub fn combine_optional_errors(
    primary: Option<String>,
    secondary: Option<String>,
) -> Option<String> {
    match primary {
        Some(primary) => Some(append_secondary_message(primary, secondary)),
        None => secondary,
    }
}

/// Unified application error used at host and adapter boundaries.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct DatabaseError(anyhow::Error);

impl DatabaseError {
    pub fn new(error: impl Into<anyhow::Error>) -> Self {
        Self(error.into())
    }
}

/// Unified application error used at host and adapter boundaries.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("内部错误：{0}")]
    Internal(anyhow::Error),

    #[error("数据库错误：{0}")]
    Database(#[from] DatabaseError),

    #[error("未找到：{0}")]
    NotFound(String),

    #[error("权限不足：{0}")]
    Forbidden(String),

    #[error("未认证：{0}")]
    Unauthorized(String),

    #[error("参数无效：{0}")]
    InvalidArgument(String),

    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },

    #[error("LLM 调用失败：{0}")]
    LlmError(String),

    #[error("向量检索失败：{0}")]
    VectorStoreError(String),

    #[error("嵌入计算失败：{0}")]
    EmbeddingError(String),

    #[error("任务已取消")]
    Cancelled,

    #[error("任务超时：{0}")]
    Timeout(String),

    #[error("配置错误：{0}")]
    ConfigError(String),

    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误：{0}")]
    Serialization(#[from] serde_json::Error),
}

impl AppError {
    pub fn database(error: impl Into<anyhow::Error>) -> Self {
        Self::Database(DatabaseError::new(error))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl From<a4n_domain::ProviderError> for AppError {
    fn from(error: a4n_domain::ProviderError) -> Self {
        match error {
            a4n_domain::ProviderError::InvalidArgument(message) => Self::InvalidArgument(message),
            a4n_domain::ProviderError::Configuration(message) => Self::ConfigError(message),
            a4n_domain::ProviderError::Provider(message)
            | a4n_domain::ProviderError::Internal(message) => {
                Self::Internal(anyhow::anyhow!(message))
            }
            a4n_domain::ProviderError::Embedding(message) => Self::EmbeddingError(message),
            a4n_domain::ProviderError::VectorStore(message) => Self::VectorStoreError(message),
            a4n_domain::ProviderError::Timeout(message) => Self::Timeout(message),
            a4n_domain::ProviderError::Cancelled => Self::Cancelled,
            a4n_domain::ProviderError::NotFound(message) => Self::NotFound(message),
        }
    }
}

impl From<a4n_domain::profile_text::ProfileTextError> for AppError {
    fn from(error: a4n_domain::profile_text::ProfileTextError) -> Self {
        Self::InvalidArgument(error.to_string())
    }
}

impl From<a4n_domain::project::ProjectStatusParseError> for AppError {
    fn from(error: a4n_domain::project::ProjectStatusParseError) -> Self {
        Self::InvalidArgument(error.to_string())
    }
}

impl From<a4n_domain::prompt_language::PromptLanguageError> for AppError {
    fn from(error: a4n_domain::prompt_language::PromptLanguageError) -> Self {
        Self::InvalidArgument(error.to_string())
    }
}
