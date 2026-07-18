pub use a4n_db::admin::{
    AdminDbError, AdminOverviewSummary, AdminUserSummary, DailyUsagePoint, GenerationCallAnalytics,
    GenerationCallAnalyticsInput, InviteCodeSummary, ProjectSummary, TaskQueueSummary, ThemeInfo,
    UsageLogSummary, UsageTrendsInput,
};

use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn admin_usage_trends(
        &self,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, AppError> {
        a4n_db::admin::usage_trends(self.db.connection(), input)
            .await
            .map_err(admin_db_error)
    }

    pub async fn admin_overview(&self) -> Result<AdminOverviewSummary, AppError> {
        a4n_db::admin::admin_overview_summary(self.db.connection())
            .await
            .map_err(admin_db_error)
    }

    pub async fn user_usage(
        &self,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, AppError> {
        a4n_db::admin::usage_by_user(self.db.connection(), user_id, limit)
            .await
            .map_err(admin_db_error)
    }

    pub async fn all_user_summaries(&self) -> Result<Vec<AdminUserSummary>, AppError> {
        a4n_db::admin::all_user_summaries(self.db.connection())
            .await
            .map_err(admin_db_error)
    }

    pub async fn invite_code_summaries(&self) -> Result<Vec<InviteCodeSummary>, AppError> {
        a4n_db::admin::invite_code_summaries(self.db.connection())
            .await
            .map_err(admin_db_error)
    }

    pub async fn token_summary(&self, user_id: i64) -> Result<serde_json::Value, AppError> {
        a4n_db::admin::token_summary(self.db.connection(), user_id)
            .await
            .map_err(admin_db_error)
    }

    pub async fn pending_tasks(&self) -> Result<Vec<TaskQueueSummary>, AppError> {
        a4n_db::admin::pending_tasks(self.db.connection())
            .await
            .map_err(admin_db_error)
    }

    pub async fn all_project_summaries(&self) -> Result<Vec<ProjectSummary>, AppError> {
        a4n_db::admin::all_project_summaries(self.db.connection())
            .await
            .map_err(admin_db_error)
    }

    pub async fn database_health(&self) -> Result<serde_json::Value, AppError> {
        a4n_db::admin::database_health(self.db.connection())
            .await
            .map_err(admin_db_error)
    }

    pub async fn generation_call_analytics(
        &self,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, AppError> {
        a4n_db::admin::generation_call_analytics(self.db.connection(), input)
            .await
            .map_err(admin_db_error)
    }
}

fn admin_db_error(error: AdminDbError) -> AppError {
    match error {
        AdminDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AdminDbError::Database(error) => AppError::Database(error),
        AdminDbError::DatabaseOperation { source, .. } => AppError::Database(source),
    }
}
