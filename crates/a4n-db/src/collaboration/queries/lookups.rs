use crate::collaboration::CollaborationDbError;
use a4n_core::models::{
    project::{novel_project, project_invite, project_member},
    user::user,
};
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter};

pub(in crate::collaboration) async fn find_user<C>(
    db: &C,
    user_id: i64,
) -> Result<user::Model, CollaborationDbError>
where
    C: ConnectionTrait,
{
    user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(CollaborationDbError::Database)?
        .ok_or_else(|| CollaborationDbError::NotFound(format!("用户不存在: {user_id}")))
}

pub(in crate::collaboration) async fn find_project<C>(
    db: &C,
    project_id: i64,
) -> Result<novel_project::Model, CollaborationDbError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(CollaborationDbError::Database)?
        .ok_or_else(|| CollaborationDbError::NotFound(format!("项目不存在: {project_id}")))
}

pub(in crate::collaboration) async fn find_user_by_identity<C>(
    db: &C,
    value: &str,
) -> Result<user::Model, CollaborationDbError>
where
    C: ConnectionTrait,
{
    let value = value.trim();
    if value.is_empty() {
        return Err(CollaborationDbError::InvalidArgument(
            "邀请对象不能为空".to_string(),
        ));
    }
    user::Entity::find()
        .filter(
            Condition::any()
                .add(user::Column::Username.eq(value))
                .add(user::Column::Email.eq(value)),
        )
        .one(db)
        .await
        .map_err(CollaborationDbError::Database)?
        .ok_or_else(|| CollaborationDbError::NotFound(format!("用户不存在: {value}")))
}

pub(in crate::collaboration) async fn find_invite<C>(
    db: &C,
    invite_id: i64,
) -> Result<project_invite::Model, CollaborationDbError>
where
    C: ConnectionTrait,
{
    project_invite::Entity::find_by_id(invite_id)
        .one(db)
        .await
        .map_err(CollaborationDbError::Database)?
        .ok_or_else(|| CollaborationDbError::NotFound(format!("项目邀请不存在: {invite_id}")))
}

pub(in crate::collaboration) async fn find_member<C>(
    db: &C,
    project_id: i64,
    user_id: i64,
) -> Result<project_member::Model, CollaborationDbError>
where
    C: ConnectionTrait,
{
    project_member::Entity::find()
        .filter(project_member::Column::ProjectId.eq(project_id))
        .filter(project_member::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(CollaborationDbError::Database)?
        .ok_or_else(|| CollaborationDbError::NotFound("项目协作者不存在".to_string()))
}
