use super::permissions::{AdminPermission, require_admin_role};
use a4n_core::error::AppError;

#[test]
fn require_admin_role_allows_admin_for_all_admin_permissions() {
    require_admin_role("admin", AdminPermission::ManageUsers).expect("admin should manage users");
    require_admin_role("admin", AdminPermission::ManageSettings)
        .expect("admin should manage settings");
}

#[test]
fn require_admin_role_rejects_non_admin_roles() {
    let err = require_admin_role("user", AdminPermission::ManageUsers)
        .expect_err("non-admin role should be rejected");

    assert!(matches!(err, AppError::Forbidden(_)));
    assert!(err.to_string().contains("ManageUsers"));
}

#[test]
fn require_admin_role_rejects_unknown_role_values() {
    let err = require_admin_role("owner", AdminPermission::ManageSettings)
        .expect_err("unknown role should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}
