use super::macros::{
    project_web_project_input_value_command, project_web_project_unit_command,
    project_web_project_value_command,
};
use crate::routes::commands::{
    ApiError, command_input, i64_arg, project_collaboration_actions, to_value,
};
use crate::state::ServerState;
use serde_json::Value;

project_web_project_value_command!(
    list_project_members,
    project_collaboration_actions::list_project_members
);
project_web_project_input_value_command!(
    invite_project_member,
    project_collaboration_actions::InviteProjectMemberInput,
    project_collaboration_actions::invite_project_member
);
project_web_project_value_command!(
    list_project_invites,
    project_collaboration_actions::list_project_invites
);

pub(in crate::routes::commands) async fn list_my_project_invites(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        project_collaboration_actions::list_my_project_invites(state.runtime(), user_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn accept_project_invite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let invite_id = i64_arg(args, "inviteId")?;
    to_value(
        project_collaboration_actions::accept_project_invite(state.runtime(), user_id, invite_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn decline_project_invite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let invite_id = i64_arg(args, "inviteId")?;
    to_value(
        project_collaboration_actions::decline_project_invite(state.runtime(), user_id, invite_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn revoke_project_invite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let invite_id = i64_arg(args, "inviteId")?;
    to_value(
        project_collaboration_actions::revoke_project_invite(
            state.runtime(),
            user_id,
            project_id,
            invite_id,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn update_project_member_role(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let member_user_id = i64_arg(args, "memberUserId")?;
    let input: project_collaboration_actions::UpdateProjectMemberRoleInput =
        command_input(args, "update_project_member_role")?;
    to_value(
        project_collaboration_actions::update_project_member_role(
            state.runtime(),
            user_id,
            project_id,
            member_user_id,
            input,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn remove_project_member(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let member_user_id = i64_arg(args, "memberUserId")?;
    project_collaboration_actions::remove_project_member(
        state.runtime(),
        user_id,
        project_id,
        member_user_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

project_web_project_unit_command!(leave_project, project_collaboration_actions::leave_project);
