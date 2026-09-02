use super::indexes::{
    projects_by_id, unique_invite_project_ids, unique_invite_user_ids, users_by_id,
};
use crate::collaboration::{CollaborationDbError, ProjectInviteSummary, parse_stored_member_role};
use a4n_core::models::project::project_invite;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

pub(in crate::collaboration) async fn invite_summaries(
    db: &DatabaseConnection,
    invites: Vec<project_invite::Model>,
) -> Result<Vec<ProjectInviteSummary>, CollaborationDbError> {
    if invites.is_empty() {
        return Ok(Vec::new());
    }
    let user_ids = unique_invite_user_ids(&invites);
    let project_ids = unique_invite_project_ids(&invites);
    let (users_by_id, projects_by_id) =
        futures::join!(users_by_id(db, &user_ids), projects_by_id(db, &project_ids),);
    let users_by_id = users_by_id?;
    let projects_by_id = projects_by_id?;
    let mut summaries = Vec::with_capacity(invites.len());
    for invite in invites {
        summaries.push(invite_summary_from_indexes(
            invite,
            &users_by_id,
            &projects_by_id,
        )?);
    }
    Ok(summaries)
}

fn invite_summary_from_indexes(
    invite: project_invite::Model,
    users_by_id: &HashMap<i64, super::super::types::UserDisplayRow>,
    projects_by_id: &HashMap<i64, String>,
) -> Result<ProjectInviteSummary, CollaborationDbError> {
    let inviter = users_by_id.get(&invite.inviter_user_id).ok_or_else(|| {
        CollaborationDbError::Internal(format!("邀请发起人不存在: {}", invite.inviter_user_id))
    })?;
    let invitee = users_by_id.get(&invite.invitee_user_id).ok_or_else(|| {
        CollaborationDbError::Internal(format!("被邀请用户不存在: {}", invite.invitee_user_id))
    })?;
    let project = projects_by_id.get(&invite.project_id).ok_or_else(|| {
        CollaborationDbError::Internal(format!("邀请项目不存在: {}", invite.project_id))
    })?;
    Ok(ProjectInviteSummary {
        id: invite.id,
        project_title: project.clone(),
        inviter_username: inviter.username.clone(),
        invitee_username: invitee.username.clone(),
        role: parse_stored_member_role(&invite.role)?,
    })
}
