use crate::routes::commands::{ApiError, project_records_actions, to_value};
use crate::routes::projects::project_summary_with_family_for_user;
use crate::state::ServerState;
use a4n_core::models::project::novel_project;
use serde_json::Value;

pub(super) async fn project_summary_value(
    state: &ServerState,
    user_id: i64,
    project: &novel_project::Model,
) -> Result<Value, ApiError> {
    to_value(project_summary_with_family_for_user(state, user_id, project).await?)
}

pub(super) async fn project_summaries_value(
    state: &ServerState,
    user_id: i64,
    projects: Vec<novel_project::Model>,
) -> Result<Value, ApiError> {
    to_value(
        project_records_actions::project_summaries_with_family_for_user(
            state.runtime(),
            user_id,
            &projects,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

macro_rules! project_web_input_summary_command {
    ($name:ident, $input:ty, $action:path) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let input: $input = $crate::routes::commands::command_input(args, stringify!($name))?;
            let project = $action(state.runtime(), user_id, input)
                .await
                .map_err($crate::routes::commands::ApiError::bad_request)?;
            super::macros::project_summary_value(state, user_id, &project).await
        }
    };
}

macro_rules! project_web_input_value_command {
    ($name:ident, $input:ty, $action:path) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let input: $input = $crate::routes::commands::command_input(args, stringify!($name))?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), user_id, input)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_project_value_command {
    ($name:ident, $action:path) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let project_id = $crate::routes::commands::i64_arg(args, "projectId")?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), user_id, project_id)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_project_node_value_command {
    ($name:ident, $action:path) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let project_id = $crate::routes::commands::i64_arg(args, "projectId")?;
            let node_id = $crate::routes::commands::string_arg(args, "nodeId")?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), user_id, project_id, node_id)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_project_unit_command {
    ($name:ident, $action:path) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let project_id = $crate::routes::commands::i64_arg(args, "projectId")?;
            $action(state.runtime(), user_id, project_id)
                .await
                .map_err($crate::routes::commands::ApiError::bad_request)?;
            Ok(serde_json::Value::Null)
        }
    };
}

macro_rules! project_web_project_input_value_command {
    ($name:ident, $input:ty, $action:path) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let project_id = $crate::routes::commands::i64_arg(args, "projectId")?;
            let input: $input = $crate::routes::commands::command_input(args, stringify!($name))?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), user_id, project_id, input)
                    .await
                    .map_err($crate::routes::commands::ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_status_command {
    ($name:ident, $action:path) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::commands::ApiError> {
            let project_id = $crate::routes::commands::i64_arg(args, "projectId")?;
            let target = serde_json::from_value::<a4n_api_contract::project::ProjectStatus>(
                $crate::routes::commands::required_arg(args, "target")?.clone(),
            )
            .map_err(|error| {
                $crate::routes::commands::ApiError::bad_request(format!(
                    "{}.target 无效: {error}",
                    stringify!($name)
                ))
            })?;
            let project = $action(state.runtime(), user_id, project_id, target)
                .await
                .map_err($crate::routes::commands::ApiError::bad_request)?;
            super::macros::project_summary_value(state, user_id, &project).await
        }
    };
}

pub(super) use project_web_input_summary_command;
pub(super) use project_web_input_value_command;
pub(super) use project_web_project_input_value_command;
pub(super) use project_web_project_node_value_command;
pub(super) use project_web_project_unit_command;
pub(super) use project_web_project_value_command;
pub(super) use project_web_status_command;
