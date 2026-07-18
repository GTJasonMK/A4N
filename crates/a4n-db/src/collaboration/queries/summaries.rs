use super::indexes::{
    projects_by_id, unique_invite_project_ids, unique_invite_user_ids, users_by_id,
};
use super::lookups::{find_project, find_user};
use crate::collaboration::{CollaborationDbError, ProjectInviteSummary, ProjectMemberSummary};
use a4n_core::models::{
    project::{novel_project, project_invite, project_member},
    user::user,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::collections::HashMap;

pub(in crate::collaboration) async fn member_summary<C>(
    db: &C,
    member: project_member::Model,
) -> Result<ProjectMemberSummary, CollaborationDbError>
where
    C: ConnectionTrait,
{
    let user = find_user(db, member.user_id).await?;
    Ok(ProjectMemberSummary {
        project_id: member.project_id,
        user_id: member.user_id,
        username: user.username,
        email: user.email,
        role: member.role,
        is_owner: false,
        invited_by_user_id: Some(member.invited_by_user_id),
        created_at: Some(member.created_at.to_string()),
        updated_at: Some(member.updated_at.to_string()),
    })
}

pub(in crate::collaboration) async fn invite_summaries(
    db: &DatabaseConnection,
    invites: Vec<project_invite::Model>,
) -> Result<Vec<ProjectInviteSummary>, CollaborationDbError> {
    if invites.is_empty() {
        return Ok(Vec::new());
    }
    let user_ids = unique_invite_user_ids(&invites);
    let project_ids = unique_invite_project_ids(&invites);
    let users_by_id = users_by_id(db, &user_ids).await?;
    let projects_by_id = projects_by_id(db, &project_ids).await?;
    let mut summaries = Vec::with_capacity(invites.len());
    for invite in invites {
        summaries.push(invite_summary_from_indexes(
            invite,
            &users_by_id,
            &projects_by_id,
        ));
    }
    Ok(summaries)
}

pub(in crate::collaboration) async fn invite_summary<C>(
    db: &C,
    invite: project_invite::Model,
) -> Result<ProjectInviteSummary, CollaborationDbError>
where
    C: ConnectionTrait,
{
    let inviter = find_user(db, invite.inviter_user_id).await?;
    let invitee = find_user(db, invite.invitee_user_id).await?;
    let project = find_project(db, invite.project_id).await?;
    Ok(ProjectInviteSummary {
        id: invite.id,
        project_id: invite.project_id,
        project_title: Some(project.title),
        inviter_user_id: invite.inviter_user_id,
        inviter_username: Some(inviter.username),
        invitee_user_id: invite.invitee_user_id,
        invitee_username: Some(invitee.username),
        invitee_email: Some(invitee.email),
        role: invite.role,
        status: invite.status,
        expires_at: invite.expires_at.to_string(),
        accepted_at: invite.accepted_at.map(|time| time.to_string()),
        created_at: Some(invite.created_at.to_string()),
        updated_at: Some(invite.updated_at.to_string()),
    })
}

pub(in crate::collaboration) fn invite_summary_from_indexes(
    invite: project_invite::Model,
    users_by_id: &HashMap<i64, user::Model>,
    projects_by_id: &HashMap<i64, novel_project::Model>,
) -> ProjectInviteSummary {
    let inviter = users_by_id.get(&invite.inviter_user_id);
    let invitee = users_by_id.get(&invite.invitee_user_id);
    let project = projects_by_id.get(&invite.project_id);
    ProjectInviteSummary {
        id: invite.id,
        project_id: invite.project_id,
        project_title: project.map(|project| project.title.clone()),
        inviter_user_id: invite.inviter_user_id,
        inviter_username: inviter.map(|user| user.username.clone()),
        invitee_user_id: invite.invitee_user_id,
        invitee_username: invitee.map(|user| user.username.clone()),
        invitee_email: invitee.map(|user| user.email.clone()),
        role: invite.role,
        status: invite.status,
        expires_at: invite.expires_at.to_string(),
        accepted_at: invite.accepted_at.map(|time| time.to_string()),
        created_at: Some(invite.created_at.to_string()),
        updated_at: Some(invite.updated_at.to_string()),
    }
}
