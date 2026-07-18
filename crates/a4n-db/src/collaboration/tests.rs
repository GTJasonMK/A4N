use super::*;

#[test]
fn collaboration_database_error_keeps_db_error_source() {
    let error = CollaborationDbError::Database(sea_orm::DbErr::Custom(
        "collaboration query failed".to_string(),
    ));

    assert!(matches!(
        &error,
        CollaborationDbError::Database(sea_orm::DbErr::Custom(message))
            if message == "collaboration query failed"
    ));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn project_access_scope_parses_optional_values() {
    assert_eq!(
        ProjectAccessScope::from_optional(None).expect("default scope"),
        ProjectAccessScope::All
    );
    assert_eq!(
        ProjectAccessScope::from_optional(Some("owned")).expect("owned scope"),
        ProjectAccessScope::Owned
    );
    assert_eq!(
        ProjectAccessScope::from_optional(Some("collaborating")).expect("collaborating scope"),
        ProjectAccessScope::Collaborating
    );
    assert!(matches!(
        ProjectAccessScope::from_optional(Some("bad")).expect_err("bad scope should fail"),
        CollaborationDbError::InvalidArgument(message) if message == "项目访问范围无效: bad"
    ));
}

#[test]
fn normalize_project_kind_keeps_existing_errors() {
    assert_eq!(
        normalize_project_kind("comic").expect("comic kind"),
        "comic"
    );
    assert_eq!(
        normalize_project_kind(" ")
            .expect_err("blank kind should fail")
            .to_string(),
        "项目族类型不能为空"
    );
    assert_eq!(
        normalize_project_kind("audio")
            .expect_err("unknown kind should fail")
            .to_string(),
        "项目族类型无效: audio"
    );
}

#[test]
fn role_satisfies_manager_matches_owner_and_manager_only() {
    assert!(role_satisfies_manager("owner"));
    assert!(role_satisfies_manager("manager"));
    assert!(!role_satisfies_manager("editor"));
    assert!(!role_satisfies_manager("viewer"));
}

#[test]
fn project_access_role_satisfies_by_rank() {
    assert!(ProjectAccessRole::Owner.satisfies(ProjectAccessRole::Editor));
    assert!(ProjectAccessRole::Manager.satisfies(ProjectAccessRole::Viewer));
    assert!(ProjectAccessRole::Editor.satisfies(ProjectAccessRole::Editor));
    assert!(!ProjectAccessRole::Viewer.satisfies(ProjectAccessRole::Editor));
    assert!(!ProjectAccessRole::Editor.satisfies(ProjectAccessRole::Manager));
}

#[test]
fn member_role_to_project_access_role_preserves_existing_errors() {
    assert_eq!(
        member_role_to_project_access_role("manager").expect("manager role"),
        ProjectAccessRole::Manager
    );
    assert_eq!(
        member_role_to_project_access_role("bad")
            .expect_err("bad role should fail")
            .to_string(),
        "未知项目协作角色: bad"
    );
}

#[test]
fn parse_project_member_role_trims_and_rejects_unknown_roles() {
    assert_eq!(
        parse_project_member_role(" editor ").expect("editor role"),
        ProjectMemberRole::Editor
    );
    assert_eq!(
        parse_project_member_role("owner")
            .expect_err("owner is not a member role")
            .to_string(),
        "未知项目协作角色: owner"
    );
}
