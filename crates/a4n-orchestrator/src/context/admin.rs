use a4n_core::error::AppError;
use a4n_core::models::user::role::{UserRole, parse_user_role};
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::admin::{
    AdminDbError, AdminOverviewSummary, AdminPermission, AdminProjectSummary, DailyUsagePoint,
    GenerationCallAnalytics, GenerationCallAnalyticsInput, TaskQueueSummary, UsageLogSummary,
    UsageTrendsInput, require_admin_role,
};

#[derive(Clone)]
pub(super) struct AdminApplication {
    database: DbHandle,
}

impl AdminApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn usage_trends(
        &self,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, AppError> {
        a4n_db::admin::usage_trends(self.database.connection(), input)
            .await
            .map_err(admin_db_error)
    }

    async fn overview(&self) -> Result<AdminOverviewSummary, AppError> {
        a4n_db::admin::admin_overview_summary(self.database.connection())
            .await
            .map_err(admin_db_error)
    }

    async fn user_usage(&self, user_id: i64, limit: u64) -> Result<Vec<UsageLogSummary>, AppError> {
        a4n_db::admin::usage_by_user(self.database.connection(), user_id, limit)
            .await
            .map_err(admin_db_error)
    }

    async fn token_summary(&self, user_id: i64) -> Result<serde_json::Value, AppError> {
        a4n_db::admin::token_summary(self.database.connection(), user_id)
            .await
            .map_err(admin_db_error)
    }

    async fn pending_tasks(&self) -> Result<Vec<TaskQueueSummary>, AppError> {
        a4n_db::admin::pending_tasks(self.database.connection())
            .await
            .map_err(admin_db_error)
    }

    async fn all_project_summaries(&self) -> Result<Vec<AdminProjectSummary>, AppError> {
        a4n_db::admin::all_project_summaries(self.database.connection())
            .await
            .map_err(admin_db_error)
    }

    async fn database_health(&self) -> Result<serde_json::Value, AppError> {
        a4n_db::admin::database_health(self.database.connection())
            .await
            .map_err(admin_db_error)
    }

    async fn generation_call_analytics(
        &self,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, AppError> {
        a4n_db::admin::generation_call_analytics(self.database.connection(), input)
            .await
            .map_err(admin_db_error)
    }
}

impl OrchestratorContext {
    pub async fn require_admin_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), AppError> {
        let user = self.auth_user_by_id(caller_user_id).await?;
        if !user.is_active {
            return Err(AppError::Forbidden(format!("用户 {caller_user_id} 已停用")));
        }
        require_admin_role(&user.role, permission)
    }

    pub async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, AppError> {
        let user = self.auth_user_by_id(user_id).await?;
        if !user.is_active {
            return Ok(false);
        }
        Ok(parse_user_role(&user.role)? == UserRole::Admin)
    }

    pub async fn admin_usage_trends(
        &self,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, AppError> {
        self.admin.usage_trends(input).await
    }

    pub async fn admin_overview(&self) -> Result<AdminOverviewSummary, AppError> {
        self.admin.overview().await
    }

    pub async fn user_usage(
        &self,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, AppError> {
        self.admin.user_usage(user_id, limit).await
    }

    pub async fn token_summary(&self, user_id: i64) -> Result<serde_json::Value, AppError> {
        self.admin.token_summary(user_id).await
    }

    pub async fn pending_tasks(&self) -> Result<Vec<TaskQueueSummary>, AppError> {
        self.admin.pending_tasks().await
    }

    pub async fn all_project_summaries(&self) -> Result<Vec<AdminProjectSummary>, AppError> {
        self.admin.all_project_summaries().await
    }

    pub async fn database_health(&self) -> Result<serde_json::Value, AppError> {
        self.admin.database_health().await
    }

    pub async fn generation_call_analytics(
        &self,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, AppError> {
        self.admin.generation_call_analytics(input).await
    }
}

fn admin_db_error(error: AdminDbError) -> AppError {
    match error {
        AdminDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AdminDbError::Database(error) => AppError::Database(error),
        AdminDbError::DatabaseOperation { source, .. } => AppError::Database(source),
    }
}
