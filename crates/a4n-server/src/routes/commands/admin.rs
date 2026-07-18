use super::{command_input, i64_arg, string_arg, to_value, u64_arg};
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
            require_admin_caller($user_id, $args)?;
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
            require_admin_caller($user_id, $args)?;
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
            require_admin_caller($user_id, $args)?;
            $body;
            Ok(Value::Null)
        }
    };
}

admin_web_json_command!(get_user_usage, |state, user_id, args| {
    let target_user_id = i64_arg(args, "userId")?;
    let limit = u64_arg(args, "limit")?;
    admin_records_actions::get_user_usage_for_admin(state.runtime(), user_id, target_user_id, limit)
        .await?
});

admin_web_value_command!(get_token_summary, |state, user_id, args| {
    let target_user_id = i64_arg(args, "userId")?;
    admin_records_actions::get_token_summary_for_admin(state.runtime(), user_id, target_user_id)
        .await?
});

admin_web_json_command!(list_pending_tasks, |state, user_id, _args| {
    admin_records_actions::list_pending_tasks_for_admin(state.runtime(), user_id).await?
});

admin_web_unit_command!(cancel_admin_task, |state, user_id, args| {
    let task_id = string_arg(args, "taskId")?;
    admin_records_actions::cancel_admin_task_for_admin(state.runtime(), user_id, &task_id).await?;
});

admin_web_value_command!(health_check, |state, user_id, _args| {
    admin_records_actions::health_check_for_admin(state.runtime(), user_id).await?
});

admin_web_json_command!(list_all_projects, |state, user_id, _args| {
    admin_records_actions::list_all_projects_for_admin(state.runtime(), user_id).await?
});

admin_web_json_command!(get_usage_trends, |state, user_id, args| {
    let input: admin_records_actions::UsageTrendsInput = command_input(args, "get_usage_trends")?;
    admin_records_actions::get_usage_trends_for_admin(state.runtime(), user_id, input).await?
});

admin_web_json_command!(get_admin_overview, |state, user_id, _args| {
    admin_records_actions::get_admin_overview_for_admin(state.runtime(), user_id).await?
});

fn require_admin_caller(authenticated_user_id: i64, args: &Value) -> Result<(), ApiError> {
    let caller_user_id = i64_arg(args, "callerUserId")?;
    if caller_user_id != authenticated_user_id {
        return Err(ApiError::forbidden("请求用户与登录用户不一致"));
    }
    Ok(())
}
