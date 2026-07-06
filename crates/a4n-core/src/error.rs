//! 统一错误类型定义

use thiserror::Error;

/// 应用全局错误类型
#[derive(Debug, Error)]
pub enum AppError {
    #[error("内部错误：{0}")]
    Internal(anyhow::Error),

    #[error("数据库错误：{0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("未找到：{0}")]
    NotFound(String),

    #[error("权限不足：{0}")]
    Forbidden(String),

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

/// 便于从 anyhow::Error 转换
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}
