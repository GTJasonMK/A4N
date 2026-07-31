#[derive(Debug, thiserror::Error)]
pub enum WorkflowReadError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("持久化工作流数据无效: {0}")]
    InvalidData(String),
    #[error("工作流读取数据库操作失败: {0}")]
    Database(#[source] sea_orm::DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowWriteError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("持久化工作流数据无效: {0}")]
    InvalidData(String),
    #[error("当前状态 {current} 不允许执行 {operation}")]
    StatusConflict { current: String, operation: String },
    #[error("工作流写入数据库操作失败: {0}")]
    Database(#[source] sea_orm::DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowCheckpointContractError {
    #[error("{0}")]
    Invalid(String),
    #[error("读取 LLM 调用断点 {call_key} 失败：missing_checkpoint_json")]
    MissingCheckpointJson { call_key: String },
    #[error("工作流断点 JSON 序列化失败: {0}")]
    Serialization(#[source] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(in crate::workflow) struct WorkflowValidationError(pub(in crate::workflow) String);

impl From<WorkflowValidationError> for WorkflowReadError {
    fn from(error: WorkflowValidationError) -> Self {
        Self::InvalidArgument(error.to_string())
    }
}

impl From<WorkflowValidationError> for WorkflowWriteError {
    fn from(error: WorkflowValidationError) -> Self {
        Self::InvalidArgument(error.to_string())
    }
}

impl From<WorkflowValidationError> for WorkflowCheckpointContractError {
    fn from(error: WorkflowValidationError) -> Self {
        Self::Invalid(error.to_string())
    }
}

impl From<WorkflowReadError> for WorkflowWriteError {
    fn from(error: WorkflowReadError) -> Self {
        match error {
            WorkflowReadError::InvalidArgument(message) => Self::InvalidArgument(message),
            WorkflowReadError::NotFound(message) => Self::NotFound(message),
            WorkflowReadError::InvalidData(message) => Self::InvalidData(message),
            WorkflowReadError::Database(source) => Self::Database(source),
        }
    }
}
