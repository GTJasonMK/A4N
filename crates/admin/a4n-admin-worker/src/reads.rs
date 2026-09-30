use super::*;
use a4n_domain::auth::{UserRole, parse_user_role};

impl WorkerState {
    pub(super) async fn require_permission(
        &self,
        request: WorkerEnvelope<AdminPermissionInput>,
    ) -> Result<OperationAck, WorkerFailure> {
        validate_request(&request, ADMIN_PERMISSION_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, request.payload.permission)
            .await?;
        Ok(OperationAck { accepted: true })
    }

    pub(super) async fn is_active_admin_user(
        &self,
        request: WorkerEnvelope<AdminUserInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_request(&request, ADMIN_ACTIVE_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        if caller != request.payload.user_id {
            return Err(WorkerFailure::invalid_request(
                "查询管理员状态必须绑定目标用户主体",
            ));
        }
        let user = a4n_db::auth::user_by_id(self.database.connection(), caller)
            .await
            .map_err(app_error)?;
        Ok(user.is_active
            && parse_user_role(&user.role).map_err(|error| app_error(error.into()))?
                == UserRole::Admin)
    }

    pub(super) async fn user_usage(
        &self,
        request: WorkerEnvelope<AdminUserUsageInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::UsageLogSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_USER_USAGE_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::admin::usage_by_user(
            self.database.connection(),
            request.payload.user_id,
            request.payload.limit,
        )
        .await
        .map_err(admin_error)
    }

    pub(super) async fn token_summary(
        &self,
        request: WorkerEnvelope<AdminUserInput>,
    ) -> Result<serde_json::Value, WorkerFailure> {
        validate_request(&request, ADMIN_TOKEN_SUMMARY_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::admin::token_summary(self.database.connection(), request.payload.user_id)
            .await
            .map_err(admin_error)
    }

    pub(super) async fn usage_trends(
        &self,
        request: WorkerEnvelope<AdminUsageTrendsInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::DailyUsagePoint>, WorkerFailure> {
        validate_request(&request, ADMIN_USAGE_TRENDS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::admin::usage_trends(self.database.connection(), request.payload.input)
            .await
            .map_err(admin_error)
    }

    pub(super) async fn overview(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<a4n_admin_worker_contract::AdminOverviewSummary, WorkerFailure> {
        validate_request(&request, ADMIN_OVERVIEW_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageSettings)
            .await?;
        a4n_db::admin::admin_overview_summary(self.database.connection())
            .await
            .map_err(admin_error)
    }

    pub(super) async fn projects(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::AdminProjectSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_PROJECTS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::admin::all_project_summaries(self.database.connection())
            .await
            .map_err(admin_error)
    }

    pub(super) async fn pending_tasks(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::TaskQueueSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_PENDING_TASKS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageSettings)
            .await?;
        a4n_db::admin::pending_tasks(self.database.connection())
            .await
            .map_err(admin_error)
    }

    pub(super) async fn generation_analytics(
        &self,
        request: WorkerEnvelope<AdminGenerationAnalyticsInput>,
    ) -> Result<a4n_admin_worker_contract::GenerationCallAnalytics, WorkerFailure> {
        validate_request(&request, ADMIN_GENERATION_ANALYTICS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageSettings)
            .await?;
        a4n_db::admin::generation_call_analytics(self.database.connection(), request.payload.input)
            .await
            .map_err(admin_error)
    }

    pub(super) async fn database_health(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<OperationAck, WorkerFailure> {
        validate_request(&request, ADMIN_DATABASE_HEALTH_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageSettings)
            .await?;
        a4n_db::admin::check_database_health(self.database.connection())
            .await
            .map_err(admin_error)?;
        Ok(OperationAck { accepted: true })
    }

    pub(super) async fn users(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::UserSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_USERS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::auth::list_users(self.database.connection())
            .await
            .map_err(app_error)
    }

    pub(super) async fn invites(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::InviteCodeSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_INVITES_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        let codes = a4n_db::auth::list_invite_codes(self.database.connection())
            .await
            .map_err(app_error)?;
        Ok(codes.iter().map(invite_code_summary).collect())
    }
}
