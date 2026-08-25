use super::*;
use crate::project::{CreateProjectRecordInput, create_project_record};
use a4n_core::models::{
    project::{project_invite, project_member},
    user::user,
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

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

#[tokio::test]
async fn collaboration_flow_keeps_reads_pure_and_mutations_explicit() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let owner = insert_user(&db, "owner").await;
    let invitee = insert_user(&db, "invitee").await;
    let expired_invitee = insert_user(&db, "expired").await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: owner.id,
            title: "协作合同测试".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("测试项目应可创建");

    let collaborating_page = list_accessible_projects(
        &db,
        AccessibleProjectListInput {
            user_id: expired_invitee.id,
            offset: 0,
            limit: 20,
            project_kind: None,
            root_project_id: None,
            access_scope: ProjectAccessScope::Collaborating,
        },
    )
    .await
    .expect("无协作项目时列表应可读取");
    assert!(collaborating_page.items.is_empty());
    assert_eq!(collaborating_page.total, 0);

    invite_project_member(
        &db,
        owner.id,
        project.id,
        &invitee.username,
        ProjectMemberRole::Editor,
    )
    .await
    .expect("有效邀请应可创建");
    let active_invite = project_invite::Entity::find()
        .filter(project_invite::Column::ProjectId.eq(project.id))
        .filter(project_invite::Column::InviteeUserId.eq(invitee.id))
        .one(&db)
        .await
        .expect("邀请查询应成功")
        .expect("有效邀请应存在");
    let expired_invite = insert_invite(
        &db,
        project.id,
        owner.id,
        expired_invitee.id,
        Utc::now().naive_utc() - Duration::minutes(1),
    )
    .await;

    let project_invites = list_project_invites(&db, owner.id, project.id)
        .await
        .expect("项目邀请列表应可读取");
    assert_eq!(project_invites.len(), 1);
    assert_eq!(project_invites[0].id, active_invite.id);
    assert_eq!(project_invites[0].role, ProjectMemberRole::Editor);
    assert!(
        list_my_project_invites(&db, expired_invitee.id)
            .await
            .expect("个人邀请列表应可读取")
            .is_empty()
    );
    assert_eq!(
        project_invite::Entity::find_by_id(expired_invite.id)
            .one(&db)
            .await
            .expect("过期邀请状态查询应成功")
            .expect("过期邀请应仍存在")
            .status,
        INVITE_STATUS_PENDING
    );

    accept_project_invite(&db, invitee.id, active_invite.id)
        .await
        .expect("有效邀请应可接受");
    let members = list_project_members(&db, invitee.id, project.id)
        .await
        .expect("协作者应可读取成员列表");
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].role, ProjectAccessRole::Owner);
    assert_eq!(members[1].role, ProjectAccessRole::Editor);
    assert_eq!(
        project_invite::Entity::find_by_id(active_invite.id)
            .one(&db)
            .await
            .expect("已接受邀请查询应成功")
            .expect("已接受邀请应存在")
            .status,
        INVITE_STATUS_ACCEPTED
    );
    assert_eq!(
        project_ids_with_permission(
            &db,
            &[project.id, project.id, i64::MAX],
            invitee.id,
            ProjectAccessRole::Editor,
        )
        .await
        .expect("批量权限筛选应成功"),
        vec![project.id]
    );
    assert!(
        project_ids_with_permission(&db, &[project.id], invitee.id, ProjectAccessRole::Manager)
            .await
            .expect("批量权限不足筛选应成功")
            .is_empty()
    );

    leave_project(&db, invitee.id, project.id)
        .await
        .expect("协作者应可离开项目");
    assert!(
        project_member::Entity::find()
            .filter(project_member::Column::ProjectId.eq(project.id))
            .filter(project_member::Column::UserId.eq(invitee.id))
            .one(&db)
            .await
            .expect("成员查询应成功")
            .is_none()
    );
    assert!(matches!(
        leave_project(&db, invitee.id, project.id)
            .await
            .expect_err("非成员不能假成功离开"),
        CollaborationDbError::NotFound(message) if message == "项目协作者不存在"
    ));
}

async fn insert_user(db: &DatabaseConnection, label: &str) -> user::Model {
    let now = Utc::now().naive_utc();
    user::ActiveModel {
        username: Set(format!("collaboration-{label}")),
        email: Set(format!("collaboration-{label}@example.test")),
        password_hash: Set("test-password-hash".to_string()),
        role: Set("user".to_string()),
        oauth_provider: Set(None),
        oauth_subject: Set(None),
        avatar_url: Set(None),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("测试用户应可写入")
}

async fn insert_invite(
    db: &DatabaseConnection,
    project_id: i64,
    inviter_user_id: i64,
    invitee_user_id: i64,
    expires_at: chrono::NaiveDateTime,
) -> project_invite::Model {
    let now = Utc::now().naive_utc();
    project_invite::ActiveModel {
        project_id: Set(project_id),
        inviter_user_id: Set(inviter_user_id),
        invitee_user_id: Set(invitee_user_id),
        role: Set(ProjectMemberRole::Viewer.as_str().to_string()),
        status: Set(INVITE_STATUS_PENDING.to_string()),
        expires_at: Set(expires_at),
        accepted_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("测试邀请应可写入")
}
