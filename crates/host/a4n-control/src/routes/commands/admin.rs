use super::{
    command_input, empty_command_args, i64_arg, reject_unknown_command_args, string_arg, to_value,
    u64_arg,
};
use crate::state::ServerState;
use a4n_runtime::host_api::admin_records as admin_records_actions;
use serde_json::Value;

use crate::routes::error::ApiError;

macro_rules! admin_web_json_command {
    ($name:ident, |$state:ident, $user_id:ident, $args:ident| $body:block) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            to_value($body)
        }
    };
}

macro_rules! admin_web_value_command {
    ($name:ident, |$state:ident, $user_id:ident, $args:ident| $body:block) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            Ok($body)
        }
    };
}

macro_rules! admin_web_unit_command {
    ($name:ident, |$state:ident, $user_id:ident, $args:ident| $body:block) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            $body;
            Ok(Value::Null)
        }
    };
}

admin_web_json_command!(get_user_usage, |state, user_id, args| {
    reject_unknown_command_args(args, "get_user_usage", &["userId", "limit"])?;
    let target_user_id = i64_arg(args, "userId")?;
    let limit = u64_arg(args, "limit")?;
    admin_records_actions::get_user_usage_for_admin(state.runtime(), user_id, target_user_id, limit)
        .await?
});

admin_web_value_command!(get_token_summary, |state, user_id, args| {
    reject_unknown_command_args(args, "get_token_summary", &["userId"])?;
    let target_user_id = i64_arg(args, "userId")?;
    admin_records_actions::get_token_summary_for_admin(state.runtime(), user_id, target_user_id)
        .await?
});

admin_web_json_command!(list_pending_tasks, |state, user_id, args| {
    empty_command_args(args, "list_pending_tasks")?;
    admin_records_actions::list_pending_tasks_for_admin(state.runtime(), user_id).await?
});

admin_web_json_command!(cancel_admin_task, |state, user_id, args| {
    reject_unknown_command_args(args, "cancel_admin_task", &["taskId"])?;
    let task_id = string_arg(args, "taskId")?;
    admin_records_actions::cancel_admin_task_for_admin(state.runtime(), user_id, &task_id).await?
});

admin_web_value_command!(health_check, |state, user_id, args| {
    empty_command_args(args, "health_check")?;
    admin_records_actions::health_check_for_admin(state.runtime(), user_id).await?
});

admin_web_json_command!(list_all_projects, |state, user_id, args| {
    empty_command_args(args, "list_all_projects")?;
    admin_records_actions::list_all_projects_for_admin(state.runtime(), user_id).await?
});

admin_web_json_command!(get_usage_trends, |state, user_id, args| {
    reject_unknown_command_args(args, "get_usage_trends", &["input"])?;
    let input: admin_records_actions::UsageTrendsInput = command_input(args, "get_usage_trends")?;
    admin_records_actions::get_usage_trends_for_admin(state.runtime(), user_id, input).await?
});

admin_web_json_command!(get_admin_overview, |state, user_id, args| {
    empty_command_args(args, "get_admin_overview")?;
    admin_records_actions::get_admin_overview_for_admin(state.runtime(), user_id).await?
});

admin_web_json_command!(list_all_users, |state, user_id, args| {
    empty_command_args(args, "list_all_users")?;
    admin_records_actions::list_all_users_for_admin(state.runtime(), user_id).await?
});

admin_web_json_command!(create_user, |state, user_id, args| {
    reject_unknown_command_args(args, "create_user", &["input"])?;
    let input: admin_records_actions::CreateUserInput = command_input(args, "create_user")?;
    admin_records_actions::create_user_for_admin(state.runtime(), user_id, input).await?
});

admin_web_json_command!(list_invite_codes, |state, user_id, args| {
    empty_command_args(args, "list_invite_codes")?;
    admin_records_actions::list_invite_codes_for_admin(state.runtime(), user_id).await?
});

admin_web_json_command!(create_invite_code, |state, user_id, args| {
    reject_unknown_command_args(args, "create_invite_code", &["input"])?;
    let input: admin_records_actions::CreateInviteCodeInput =
        command_input(args, "create_invite_code")?;
    admin_records_actions::create_invite_code_for_admin(state.runtime(), user_id, input).await?
});

admin_web_json_command!(update_user_role, |state, user_id, args| {
    reject_unknown_command_args(args, "update_user_role", &["userId", "input"])?;
    let target_user_id = i64_arg(args, "userId")?;
    let input: admin_records_actions::UpdateUserRoleInput =
        command_input(args, "update_user_role")?;
    admin_records_actions::update_user_role_for_admin(
        state.runtime(),
        user_id,
        target_user_id,
        input,
    )
    .await?
});

admin_web_unit_command!(disable_user, |state, user_id, args| {
    reject_unknown_command_args(args, "disable_user", &["userId"])?;
    let target_user_id = i64_arg(args, "userId")?;
    admin_records_actions::disable_user_for_admin(state.runtime(), user_id, target_user_id).await?;
});
