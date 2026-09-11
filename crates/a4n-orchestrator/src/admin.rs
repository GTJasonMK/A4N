use a4n_db_models::models::user::role::{UserRole, parse_user_role};
use a4n_error::AppError;

pub use a4n_api_contract::admin::AdminPermission;

pub(crate) fn require_admin_role(role: &str, permission: AdminPermission) -> Result<(), AppError> {
    let role = parse_user_role(role)?;
    if role == UserRole::Admin {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "角色 {role:?} 无权执行操作 {permission:?}"
        )))
    }
}

pub use a4n_db::admin::{
    AdminDbError, AdminOverviewSummary, AdminProjectSummary, DailyUsagePoint,
    GenerationCallAnalytics, GenerationCallAnalyticsInput, TaskQueueSummary, ThemeInfo,
    UsageLogSummary, UsageTrendsInput,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_admin_role_allows_admin_for_all_permissions() {
        require_admin_role("admin", AdminPermission::ManageUsers).expect("admin users");
        require_admin_role("admin", AdminPermission::ManageSettings).expect("admin settings");
    }

    #[test]
    fn require_admin_role_rejects_non_admin_and_unknown_roles() {
        assert!(matches!(
            require_admin_role("user", AdminPermission::ManageUsers),
            Err(AppError::Forbidden(_))
        ));
        assert!(matches!(
            require_admin_role("owner", AdminPermission::ManageSettings),
            Err(AppError::InvalidArgument(_))
        ));
    }
}
