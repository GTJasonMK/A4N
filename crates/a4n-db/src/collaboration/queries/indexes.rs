use crate::collaboration::CollaborationDbError;
use crate::collaboration::types::INVITE_STATUS_PENDING;
use a4n_core::models::{
    project::{novel_project, project_invite, project_member},
    user::user,
};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::collections::HashMap;

pub(in crate::collaboration) fn accessible_member_condition(root_ids: &[i64]) -> Condition {
    if root_ids.is_empty() {
        Condition::all().add(Expr::value(false))
    } else {
        Condition::any()
            .add(novel_project::Column::Id.is_in(root_ids.to_vec()))
            .add(novel_project::Column::RootProjectId.is_in(root_ids.to_vec()))
    }
}

pub(in crate::collaboration) async fn member_root_ids(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Vec<i64>, CollaborationDbError> {
    let members = project_member::Entity::find()
        .filter(project_member::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    Ok(members
        .into_iter()
        .map(|member| member.project_id)
        .collect())
}

pub(in crate::collaboration) async fn users_by_id<C>(
    db: &C,
    user_ids: &[i64],
) -> Result<HashMap<i64, user::Model>, CollaborationDbError>
where
    C: ConnectionTrait,
{
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = user::Entity::find()
        .filter(user::Column::Id.is_in(user_ids.iter().copied()))
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

pub(in crate::collaboration) async fn projects_by_id<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<HashMap<i64, novel_project::Model>, CollaborationDbError>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = novel_project::Entity::find()
        .filter(novel_project::Column::Id.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

pub(in crate::collaboration) async fn pending_invite_counts_by_root(
    db: &DatabaseConnection,
    root_ids: &[i64],
) -> Result<HashMap<i64, u64>, CollaborationDbError> {
    if root_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let invites = project_invite::Entity::find()
        .filter(project_invite::Column::ProjectId.is_in(root_ids.iter().copied()))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .filter(project_invite::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    let mut counts = HashMap::new();
    for invite in invites {
        *counts.entry(invite.project_id).or_insert(0) += 1;
    }
    Ok(counts)
}

pub(in crate::collaboration) fn unique_member_user_ids(
    owner_user_id: i64,
    members: &[project_member::Model],
) -> Vec<i64> {
    let mut ids = Vec::with_capacity(members.len() + 1);
    ids.push(owner_user_id);
    for member in members {
        ids.push(member.user_id);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub(super) fn unique_invite_user_ids(invites: &[project_invite::Model]) -> Vec<i64> {
    let mut ids = Vec::with_capacity(invites.len() * 2);
    for invite in invites {
        ids.push(invite.inviter_user_id);
        ids.push(invite.invitee_user_id);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub(super) fn unique_invite_project_ids(invites: &[project_invite::Model]) -> Vec<i64> {
    let mut ids = Vec::with_capacity(invites.len());
    for invite in invites {
        ids.push(invite.project_id);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}
