use super::{
    AdminOverviewSummary, AdminPermission, AdminProjectSummary, CreateInviteCodeInput,
    CreateUserInput, DailyUsagePoint, InviteCodeSummary, TaskQueueSummary, UpdateUserRoleInput,
    UsageLogSummary, UsageTrendsInput, UserSummary, permissions, projects, tasks, usage, users,
};
use crate::AppState;
use a4n_error::AppError;
use serde_json::Value;

macro_rules! admin_record_command {
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $output:ty,
        $permission:expr,
        |$state:ident, $caller_user_id:ident| $run:expr $(,)?
    ) => {
        pub async fn $name(
            $state: &AppState,
            $caller_user_id: i64,
            $($arg: $arg_ty,)*
        ) -> Result<$output, AppError> {
            permissions::require_admin_permission($state, $caller_user_id, $permission).await?;
            $run.await.map_err(AppError::InvalidArgument)
        }
    };
}

admin_record_command!(
    get_user_usage_for_admin(user_id: i64, limit: u64) -> Vec<UsageLogSummary>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| usage::get_user_usage(state, caller_user_id, user_id, limit),
);

admin_record_command!(
    get_token_summary_for_admin(user_id: i64) -> Value,
    AdminPermission::ManageUsers,
    |state, caller_user_id| usage::get_token_summary(state, caller_user_id, user_id),
);

admin_record_command!(
    list_pending_tasks_for_admin() -> Vec<TaskQueueSummary>,
    AdminPermission::ManageSettings,
    |state, caller_user_id| tasks::list_pending_tasks(state, caller_user_id),
);

admin_record_command!(
    cancel_admin_task_for_admin(task_id: &str) -> (),
    AdminPermission::ManageSettings,
    |state, caller_user_id| tasks::cancel_admin_task(state, task_id),
);

admin_record_command!(
    health_check_for_admin() -> Value,
    AdminPermission::ManageSettings,
    |state, caller_user_id| tasks::health_check(state, caller_user_id),
);

admin_record_command!(
    list_all_users_for_admin() -> Vec<UserSummary>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| users::list_all_users(state, caller_user_id),
);

admin_record_command!(
    create_user_for_admin(input: CreateUserInput) -> UserSummary,
    AdminPermission::ManageUsers,
    |state, caller_user_id| users::create_user(state, caller_user_id, input),
);

admin_record_command!(
    list_invite_codes_for_admin() -> Vec<InviteCodeSummary>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| users::list_invite_codes(state, caller_user_id),
);

admin_record_command!(
    create_invite_code_for_admin(input: CreateInviteCodeInput) -> InviteCodeSummary,
    AdminPermission::ManageUsers,
    |state, caller_user_id| users::create_invite_code(state, caller_user_id, input),
);

admin_record_command!(
    update_user_role_for_admin(user_id: i64, input: UpdateUserRoleInput) -> UserSummary,
    AdminPermission::ManageUsers,
    |state, caller_user_id| users::update_user_role(state, caller_user_id, user_id, input),
);

admin_record_command!(
    disable_user_for_admin(user_id: i64) -> (),
    AdminPermission::ManageUsers,
    |state, caller_user_id| users::disable_user(state, caller_user_id, user_id),
);

admin_record_command!(
    list_all_projects_for_admin() -> Vec<AdminProjectSummary>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| projects::list_all_projects(state, caller_user_id),
);

admin_record_command!(
    get_usage_trends_for_admin(input: UsageTrendsInput) -> Vec<DailyUsagePoint>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| usage::get_usage_trends(state, caller_user_id, input),
);

admin_record_command!(
    get_admin_overview_for_admin() -> AdminOverviewSummary,
    AdminPermission::ManageSettings,
    |state, caller_user_id| usage::get_admin_overview(state, caller_user_id),
);
