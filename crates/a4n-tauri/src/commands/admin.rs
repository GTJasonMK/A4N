//! 管理后台命令
//!
//! 暴露用量统计、任务队列管理、用户管理、项目管理和系统配置接口。

use a4n_runtime::AppState;
use a4n_runtime::host_api::admin_records;
use tauri::State;

use admin_records::{
    AdminUserSummary, CreateInviteCodeInput, CreateUserInput, DailyUsagePoint, InviteCodeSummary,
    ProjectSummary, TaskQueueSummary, UpdateUserRoleInput, UsageLogSummary, UsageTrendsInput,
};

macro_rules! admin_command {
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $output:ty,
        |$state:ident, $caller_user_id:ident| $run:expr $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            $state: State<'_, AppState>,
            $caller_user_id: i64,
            $($arg: $arg_ty,)*
        ) -> Result<$output, String> {
            $run.await.map_err(|err| err.to_string())
        }
    };
}

admin_command!(
    get_user_usage(user_id: i64, limit: u64) -> Vec<UsageLogSummary>,
    |state, caller_user_id| admin_records::get_user_usage_for_admin(
        &state,
        caller_user_id,
        user_id,
        limit,
    ),
);

admin_command!(
    get_token_summary(user_id: i64) -> serde_json::Value,
    |state, caller_user_id| admin_records::get_token_summary_for_admin(
        &state,
        caller_user_id,
        user_id,
    ),
);

admin_command!(
    list_pending_tasks() -> Vec<TaskQueueSummary>,
    |state, caller_user_id| admin_records::list_pending_tasks_for_admin(&state, caller_user_id),
);

admin_command!(
    cancel_admin_task(task_id: String) -> (),
    |state, caller_user_id| admin_records::cancel_admin_task_for_admin(
        &state,
        caller_user_id,
        &task_id,
    ),
);

admin_command!(
    health_check() -> serde_json::Value,
    |state, caller_user_id| admin_records::health_check_for_admin(&state, caller_user_id),
);

admin_command!(
    list_all_users() -> Vec<AdminUserSummary>,
    |state, caller_user_id| admin_records::list_all_users_for_admin(&state, caller_user_id),
);

admin_command!(
    create_user(input: CreateUserInput) -> AdminUserSummary,
    |state, caller_user_id| admin_records::create_user_for_admin(&state, caller_user_id, input),
);

admin_command!(
    list_invite_codes() -> Vec<InviteCodeSummary>,
    |state, caller_user_id| admin_records::list_invite_codes_for_admin(&state, caller_user_id),
);

admin_command!(
    create_invite_code(input: CreateInviteCodeInput) -> InviteCodeSummary,
    |state, caller_user_id| admin_records::create_invite_code_for_admin(
        &state,
        caller_user_id,
        input,
    ),
);

admin_command!(
    update_user_role(user_id: i64, input: UpdateUserRoleInput) -> AdminUserSummary,
    |state, caller_user_id| admin_records::update_user_role_for_admin(
        &state,
        caller_user_id,
        user_id,
        input,
    ),
);

admin_command!(
    delete_user(user_id: i64) -> (),
    |state, caller_user_id| admin_records::delete_user_for_admin(&state, caller_user_id, user_id),
);

admin_command!(
    list_all_projects() -> Vec<ProjectSummary>,
    |state, caller_user_id| admin_records::list_all_projects_for_admin(&state, caller_user_id),
);

admin_command!(
    get_usage_trends(input: UsageTrendsInput) -> Vec<DailyUsagePoint>,
    |state, caller_user_id| admin_records::get_usage_trends_for_admin(
        &state,
        caller_user_id,
        input,
    ),
);

admin_command!(
    get_admin_overview() -> admin_records::AdminOverviewSummary,
    |state, caller_user_id| admin_records::get_admin_overview_for_admin(&state, caller_user_id),
);
