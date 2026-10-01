use a4n_admin_worker_contract::{
    ADMIN_ACTIVE_OPERATION, ADMIN_DATABASE_HEALTH_OPERATION, ADMIN_GENERATION_ANALYTICS_OPERATION,
    ADMIN_INVITES_OPERATION, ADMIN_OVERVIEW_OPERATION, ADMIN_PENDING_TASKS_OPERATION,
    ADMIN_PERMISSION_OPERATION, ADMIN_PROJECTS_OPERATION, ADMIN_TOKEN_SUMMARY_OPERATION,
    ADMIN_USAGE_TRENDS_OPERATION, ADMIN_USER_USAGE_OPERATION, ADMIN_USERS_OPERATION,
    AdminCallerInput, AdminGenerationAnalyticsInput, AdminPermissionInput, AdminUsageTrendsInput,
    AdminUserInput, AdminUserUsageInput,
};
use a4n_api_contract::admin::{
    AdminOverviewSummary, AdminPermission, AdminProjectSummary, AdminReadApplicationPort,
    DailyUsagePoint, GenerationCallAnalytics, GenerationCallAnalyticsInput, InviteCodeSummary,
    TaskQueueSummary, UsageLogSummary, UsageTrendsInput,
};
use a4n_api_contract::auth::UserSummary;
use a4n_error::AppError;

use super::AdminWorkerClient;

#[async_trait::async_trait]
impl AdminReadApplicationPort for AdminWorkerClient {
    async fn require_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), AppError> {
        self.command::<_, a4n_admin_worker_contract::OperationAck>(
            caller_user_id,
            ADMIN_PERMISSION_OPERATION,
            AdminPermissionInput {
                caller_user_id,
                permission,
            },
        )
        .await
        .map(|_| ())
    }

    async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, AppError> {
        self.command(
            user_id,
            ADMIN_ACTIVE_OPERATION,
            AdminUserInput {
                caller_user_id: user_id,
                user_id,
            },
        )
        .await
    }

    async fn user_usage(
        &self,
        caller_user_id: i64,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_USER_USAGE_OPERATION,
            AdminUserUsageInput {
                caller_user_id,
                user_id,
                limit,
            },
        )
        .await
    }

    async fn token_summary(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        self.command(
            caller_user_id,
            ADMIN_TOKEN_SUMMARY_OPERATION,
            AdminUserInput {
                caller_user_id,
                user_id,
            },
        )
        .await
    }

    async fn usage_trends(
        &self,
        caller_user_id: i64,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_USAGE_TRENDS_OPERATION,
            AdminUsageTrendsInput {
                caller_user_id,
                input,
            },
        )
        .await
    }

    async fn overview(&self, caller_user_id: i64) -> Result<AdminOverviewSummary, AppError> {
        self.command(
            caller_user_id,
            ADMIN_OVERVIEW_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }

    async fn all_project_summaries(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<AdminProjectSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_PROJECTS_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }

    async fn pending_tasks(&self, caller_user_id: i64) -> Result<Vec<TaskQueueSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_PENDING_TASKS_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }

    async fn generation_call_analytics(
        &self,
        caller_user_id: i64,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, AppError> {
        self.command(
            caller_user_id,
            ADMIN_GENERATION_ANALYTICS_OPERATION,
            AdminGenerationAnalyticsInput {
                caller_user_id,
                input,
            },
        )
        .await
    }

    async fn check_database_health(&self, caller_user_id: i64) -> Result<(), AppError> {
        self.command::<_, a4n_admin_worker_contract::OperationAck>(
            caller_user_id,
            ADMIN_DATABASE_HEALTH_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
        .map(|_| ())
    }

    async fn list_users(&self, caller_user_id: i64) -> Result<Vec<UserSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_USERS_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }

    async fn list_invite_codes(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<InviteCodeSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_INVITES_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }
}
