mod authorized;
mod permissions;
mod projects;
mod tasks;
mod test_console;
mod usage;
mod users;

pub use a4n_api_contract::admin::{
    AdminOverviewSummary, AdminProjectSummary, DailyUsagePoint, DeleteTestRunInput,
    ListTestRunsInput, StartTestRunInput, TaskQueueSummary, TestRunDetail, TestRunIdInput,
    TestRunStatus, TestRunSummary, TestScenarioDescriptor, TestTarget, TestTargetMode,
    UsageLogSummary, UsageTrendsInput,
};
pub use a4n_api_contract::admin::{CreateInviteCodeInput, CreateUserInput, UpdateUserRoleInput};
pub use a4n_api_contract::auth::UserSummary;
pub use a4n_domain::admin::InviteCodeSummary;
pub use authorized::{
    cancel_admin_task_for_admin, cancel_test_run_for_admin, create_invite_code_for_admin,
    create_user_for_admin, delete_test_run_for_admin, disable_user_for_admin,
    get_admin_overview_for_admin, get_test_run_for_admin, get_token_summary_for_admin,
    get_usage_trends_for_admin, get_user_usage_for_admin, health_check_for_admin,
    list_all_projects_for_admin, list_all_users_for_admin, list_invite_codes_for_admin,
    list_pending_tasks_for_admin, list_test_runs_for_admin, list_test_scenarios_for_admin,
    start_test_run_for_admin, update_user_role_for_admin,
};
pub use test_console::TestConsole;

pub(crate) use permissions::{AdminPermission, is_active_admin_user, require_admin_permission};
