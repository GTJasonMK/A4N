//! Wire contracts for the administrative application owner.

pub use a4n_api_contract::admin::{
    AdminOverviewSummary, AdminPermission, AdminProjectSummary, CreateInviteCodeInput,
    CreateUserInput, DailyUsagePoint, GenerationCallAnalytics, GenerationCallAnalyticsInput,
    InviteCodeSummary, PendingTaskIndexSummary, TaskQueueSummary, UpdateUserRoleInput,
    UsageLogSummary, UsageTrendsInput,
};
pub use a4n_api_contract::auth::UserSummary;
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const ADMIN_WORKER_ID: &str = "admin-worker";
pub const ADMIN_WORKER_CONTRACT_VERSION: &str = "admin-v1";
pub const ADMIN_READ_CAPABILITY: &str = "admin.read";
pub const ADMIN_WRITE_CAPABILITY: &str = "admin.write";

pub const ADMIN_PERMISSION_OPERATION: &str = "admin.permission.require";
pub const ADMIN_ACTIVE_OPERATION: &str = "admin.user.active";
pub const ADMIN_USER_USAGE_OPERATION: &str = "admin.usage.user";
pub const ADMIN_TOKEN_SUMMARY_OPERATION: &str = "admin.token.summary";
pub const ADMIN_USAGE_TRENDS_OPERATION: &str = "admin.usage.trends";
pub const ADMIN_OVERVIEW_OPERATION: &str = "admin.overview";
pub const ADMIN_PROJECTS_OPERATION: &str = "admin.projects.list";
pub const ADMIN_PENDING_TASKS_OPERATION: &str = "admin.tasks.pending";
pub const ADMIN_PENDING_TASK_INDEX_OPERATION: &str = "admin.tasks.pending.index";
pub const ADMIN_GENERATION_ANALYTICS_OPERATION: &str = "admin.generation.analytics";
pub const ADMIN_DATABASE_HEALTH_OPERATION: &str = "admin.database.health";
pub const ADMIN_USERS_OPERATION: &str = "admin.users.list";
pub const ADMIN_INVITES_OPERATION: &str = "admin.invites.list";
pub const ADMIN_CREATE_USER_OPERATION: &str = "admin.users.create";
pub const ADMIN_CREATE_INVITE_OPERATION: &str = "admin.invites.create";
pub const ADMIN_UPDATE_ROLE_OPERATION: &str = "admin.users.role.update";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminCallerInput {
    pub caller_user_id: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminPermissionInput {
    pub caller_user_id: i64,
    pub permission: AdminPermission,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminUserInput {
    pub caller_user_id: i64,
    pub user_id: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminUserUsageInput {
    pub caller_user_id: i64,
    pub user_id: i64,
    pub limit: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminUsageTrendsInput {
    pub caller_user_id: i64,
    pub input: UsageTrendsInput,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminGenerationAnalyticsInput {
    pub caller_user_id: i64,
    pub input: GenerationCallAnalyticsInput,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminCreateUserInput {
    pub caller_user_id: i64,
    pub input: CreateUserInput,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminCreateInviteInput {
    pub caller_user_id: i64,
    pub input: CreateInviteCodeInput,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminUpdateRoleInput {
    pub caller_user_id: i64,
    pub user_id: i64,
    pub input: UpdateUserRoleInput,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationAck {
    pub accepted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn admin_request_keeps_caller_inside_the_payload() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: ADMIN_OVERVIEW_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: ADMIN_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: AdminCallerInput { caller_user_id: 7 },
        };

        request.validate().expect("admin request validates");
    }
}
