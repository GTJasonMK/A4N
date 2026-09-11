use sea_orm::DbErr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultProviderConcurrencyLimits {
    pub llm: usize,
    pub image: usize,
    pub video: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum QueueDbError {
    #[error("{0}")]
    InvalidConfig(String),
    #[error("{0}")]
    NotFound(String),
    #[error("功能包 {package_id} 当前不允许创建任务: reason_code={reason_code}")]
    AdmissionBlocked {
        package_id: String,
        reason_code: String,
    },
    #[error(transparent)]
    Database(#[from] DbErr),
}
