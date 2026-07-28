mod authorized;
mod permissions;
mod projects;
mod tasks;
mod usage;
mod users;

pub use a4n_api_contract::admin::{CreateInviteCodeInput, CreateUserInput, UpdateUserRoleInput};
pub use a4n_orchestrator::admin::{
    AdminOverviewSummary, AdminUserSummary, DailyUsagePoint, InviteCodeSummary, ProjectSummary,
    TaskQueueSummary, ThemeInfo, UsageLogSummary, UsageTrendsInput,
};
pub use authorized::*;
pub use projects::*;
pub use tasks::*;
pub use usage::*;
pub use users::*;

pub(crate) use permissions::{AdminPermission, is_active_admin_user, require_admin_permission};
