use a4n_db::DbHandle;
use a4n_db::admin::{
    AdminDbError, AdminOverviewSummary, AdminProjectSummary, DailyUsagePoint,
    GenerationCallAnalytics, GenerationCallAnalyticsInput, TaskQueueSummary, UsageLogSummary,
    UsageTrendsInput,
};
use a4n_domain::admin::InviteCodeSummary;
use a4n_domain::auth::{UserRole, parse_user_role};
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::admin::{AdminPermission, require_admin_role};

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

    async fn check_database_health(&self) -> Result<(), AppError> {
        a4n_db::admin::check_database_health(self.database.connection())
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
    pub(crate) async fn require_admin_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), AppError> {
        let user = self.auth_user_authorization_by_id(caller_user_id).await?;
        if !user.is_active {
            return Err(AppError::Forbidden(format!("用户 {caller_user_id} 已停用")));
        }
        require_admin_role(&user.role, permission)
    }

    pub(crate) async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, AppError> {
        let user = self.auth_user_authorization_by_id(user_id).await?;
        if !user.is_active {
            return Ok(false);
        }
        Ok(parse_user_role(&user.role).map_err(AppError::InvalidArgument)? == UserRole::Admin)
    }

    pub(crate) async fn admin_usage_trends(
        &self,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, AppError> {
        self.admin.usage_trends(input).await
    }

    pub(crate) async fn admin_overview(&self) -> Result<AdminOverviewSummary, AppError> {
        self.admin.overview().await
    }

    pub(crate) async fn user_usage(
        &self,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, AppError> {
        self.admin.user_usage(user_id, limit).await
    }

    pub(crate) async fn token_summary(&self, user_id: i64) -> Result<serde_json::Value, AppError> {
        self.admin.token_summary(user_id).await
    }

    pub(crate) async fn pending_tasks(&self) -> Result<Vec<TaskQueueSummary>, AppError> {
        self.admin.pending_tasks().await
    }

    pub(crate) async fn all_project_summaries(&self) -> Result<Vec<AdminProjectSummary>, AppError> {
        self.admin.all_project_summaries().await
    }

    pub(crate) async fn check_database_health(&self) -> Result<(), AppError> {
        self.admin.check_database_health().await
    }

    pub(crate) async fn generation_call_analytics(
        &self,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, AppError> {
        self.admin.generation_call_analytics(input).await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::admin::AdminReadApplicationPort for OrchestratorContext {
    async fn require_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), AppError> {
        self.require_admin_permission(caller_user_id, permission)
            .await
    }

    async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, AppError> {
        OrchestratorContext::is_active_admin_user(self, user_id).await
    }

    async fn user_usage(
        &self,
        caller_user_id: i64,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        OrchestratorContext::user_usage(self, user_id, limit).await
    }

    async fn token_summary(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        OrchestratorContext::token_summary(self, user_id).await
    }

    async fn usage_trends(
        &self,
        caller_user_id: i64,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        OrchestratorContext::admin_usage_trends(self, input).await
    }

    async fn overview(&self, caller_user_id: i64) -> Result<AdminOverviewSummary, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageSettings)
            .await?;
        OrchestratorContext::admin_overview(self).await
    }

    async fn all_project_summaries(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<AdminProjectSummary>, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        OrchestratorContext::all_project_summaries(self).await
    }

    async fn pending_tasks(&self, caller_user_id: i64) -> Result<Vec<TaskQueueSummary>, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageSettings)
            .await?;
        OrchestratorContext::pending_tasks(self).await
    }

    async fn generation_call_analytics(
        &self,
        caller_user_id: i64,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageSettings)
            .await?;
        OrchestratorContext::generation_call_analytics(self, input).await
    }

    async fn check_database_health(&self, caller_user_id: i64) -> Result<(), AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageSettings)
            .await?;
        OrchestratorContext::check_database_health(self).await
    }

    async fn list_users(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<a4n_api_contract::auth::UserSummary>, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        OrchestratorContext::list_auth_users(self).await
    }

    async fn list_invite_codes(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<InviteCodeSummary>, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        OrchestratorContext::list_invite_code_records(self).await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::admin::AdminMutationApplicationPort for OrchestratorContext {
    async fn create_user(
        &self,
        caller_user_id: i64,
        input: a4n_api_contract::admin::CreateUserInput,
    ) -> Result<a4n_api_contract::auth::UserSummary, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        let user = OrchestratorContext::create_auth_user(
            self,
            &input.username,
            &input.email,
            &input.password,
            &input.role,
        )
        .await?;
        Ok(user)
    }

    async fn create_invite_code(
        &self,
        caller_user_id: i64,
        input: a4n_api_contract::admin::CreateInviteCodeInput,
    ) -> Result<InviteCodeSummary, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        let code =
            OrchestratorContext::create_invite_code_record(self, &input.code, Some(caller_user_id))
                .await?;
        Ok(code)
    }

    async fn update_user_role(
        &self,
        caller_user_id: i64,
        user_id: i64,
        input: a4n_api_contract::admin::UpdateUserRoleInput,
    ) -> Result<a4n_api_contract::auth::UserSummary, AppError> {
        self.require_admin_permission(caller_user_id, AdminPermission::ManageUsers)
            .await?;
        let user = OrchestratorContext::update_auth_user_role(self, user_id, &input.role).await?;
        Ok(user)
    }
}

fn admin_db_error(error: AdminDbError) -> AppError {
    match error {
        AdminDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AdminDbError::Database(error) => AppError::Database(error),
        AdminDbError::DatabaseOperation { source, .. } => AppError::Database(source),
    }
}
