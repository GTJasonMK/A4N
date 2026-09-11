mod authorized;
mod permissions;
mod projects;
mod tasks;
mod usage;
mod users;

pub use a4n_api_contract::admin::{
    AdminOverviewSummary, AdminProjectSummary, DailyUsagePoint, TaskQueueSummary, UsageLogSummary,
    UsageTrendsInput,
};
pub use a4n_api_contract::admin::{CreateInviteCodeInput, CreateUserInput, UpdateUserRoleInput};
pub use a4n_api_contract::auth::UserSummary;
pub use a4n_domain::admin::InviteCodeSummary;
pub use authorized::{
    cancel_admin_task_for_admin, create_invite_code_for_admin, create_user_for_admin,
    disable_user_for_admin, get_admin_overview_for_admin, get_token_summary_for_admin,
    get_usage_trends_for_admin, get_user_usage_for_admin, health_check_for_admin,
    list_all_projects_for_admin, list_all_users_for_admin, list_invite_codes_for_admin,
    list_pending_tasks_for_admin, update_user_role_for_admin,
};

pub(crate) use permissions::{AdminPermission, is_active_admin_user, require_admin_permission};
