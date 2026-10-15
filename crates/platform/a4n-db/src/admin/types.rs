use sea_orm::DbErr;

pub use a4n_domain::admin::{
    AdminOverviewSummary, AdminProjectSummary, DailyUsagePoint, GenerationCallAnalytics,
    GenerationCallAnalyticsInput, InviteCodeSummary, PendingTaskIndexSummary, TaskQueueSummary,
    ThemeInfo, UsageLogSummary, UsageTrendsInput,
};

#[derive(Debug, thiserror::Error)]
pub enum AdminDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error("{operation}: {source}")]
    DatabaseOperation {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}
