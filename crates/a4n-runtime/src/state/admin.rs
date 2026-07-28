use a4n_core::{
    error::AppError,
    models::user::{invite_code, user},
};
use a4n_orchestrator::{
    ManagedServiceHealthSummary, OrchestratorContext,
    admin::{
        AdminOverviewSummary, AdminPermission, AdminUserSummary, DailyUsagePoint,
        GenerationCallAnalytics, GenerationCallAnalyticsInput, InviteCodeSummary, ProjectSummary,
        TaskQueueSummary, UsageLogSummary, UsageTrendsInput,
    },
};

pub(crate) struct AdminRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> AdminRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn require_admin_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), AppError> {
        self.orchestrator
            .require_admin_permission(caller_user_id, permission)
            .await
    }

    pub(crate) async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, AppError> {
        self.orchestrator.is_active_admin_user(user_id).await
    }

    pub(crate) async fn usage_trends(
        &self,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, AppError> {
        self.orchestrator.admin_usage_trends(input).await
    }

    pub(crate) async fn overview(&self) -> Result<AdminOverviewSummary, AppError> {
        self.orchestrator.admin_overview().await
    }

    pub(crate) async fn user_usage(
        &self,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, AppError> {
        self.orchestrator.user_usage(user_id, limit).await
    }

    pub(crate) async fn all_user_summaries(&self) -> Result<Vec<AdminUserSummary>, AppError> {
        self.orchestrator.all_user_summaries().await
    }

    pub(crate) async fn invite_code_summaries(&self) -> Result<Vec<InviteCodeSummary>, AppError> {
        self.orchestrator.invite_code_summaries().await
    }

    pub(crate) async fn token_summary(&self, user_id: i64) -> Result<serde_json::Value, AppError> {
        self.orchestrator.token_summary(user_id).await
    }

    pub(crate) async fn pending_tasks(&self) -> Result<Vec<TaskQueueSummary>, AppError> {
        self.orchestrator.pending_tasks().await
    }

    pub(crate) async fn all_project_summaries(&self) -> Result<Vec<ProjectSummary>, AppError> {
        self.orchestrator.all_project_summaries().await
    }

    pub(crate) async fn database_health(&self) -> Result<serde_json::Value, AppError> {
        self.orchestrator.database_health().await
    }

    pub(crate) async fn generation_call_analytics(
        &self,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, AppError> {
        self.orchestrator.generation_call_analytics(input).await
    }

    pub(crate) async fn create_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<user::Model, AppError> {
        self.orchestrator
            .create_auth_user(username, email, password, role)
            .await
    }

    pub(crate) async fn create_invite_code(
        &self,
        code: &str,
        created_by_user_id: Option<i64>,
    ) -> Result<invite_code::Model, AppError> {
        self.orchestrator
            .create_invite_code_record(code, created_by_user_id)
            .await
    }

    pub(crate) async fn update_user_role(
        &self,
        user_id: i64,
        role: &str,
    ) -> Result<user::Model, AppError> {
        self.orchestrator.update_auth_user_role(user_id, role).await
    }

    pub(crate) async fn managed_service_health_summaries(
        &self,
    ) -> Result<Vec<ManagedServiceHealthSummary>, AppError> {
        self.orchestrator.managed_service_health_summaries().await
    }
}
