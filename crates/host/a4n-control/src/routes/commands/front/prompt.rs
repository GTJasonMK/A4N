use a4n_api_contract::prompt::{
    PromptAssemblyInput, RemoveOverrideInput, RenderInput, SavePromptAssemblyInput,
    SetOverrideInput, UpdatePromptInput, ValidateVarsInput, VersionDiffInput,
};
use serde_json::Value;

use super::super::{
    ApiError, ServerState, command_input, i64_arg, prompt_records_actions, string_arg, to_value,
};

macro_rules! prompt_web_no_input_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            _user_id: i64,
            _args: &Value,
        ) -> Result<Value, ApiError> {
            to_value(
                $action(state.runtime())
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! prompt_web_user_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value(
                $action(state.runtime(), user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! prompt_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            _user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value(
                $action(state.runtime(), input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! prompt_web_name_sync_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            _user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let name = string_arg(args, "name")?;
            to_value(
                $action(state.runtime(), &name)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! prompt_web_name_unit_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            _user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let name = string_arg(args, "name")?;
            $action(state.runtime(), &name)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

macro_rules! prompt_web_user_input_unit_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            $action(state.runtime(), user_id, input)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

pub(in crate::routes::commands) async fn prompt_list_project_overrides(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        prompt_records_actions::prompt_list_project_overrides(state.runtime(), user_id, project_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

prompt_web_user_input_command!(
    prompt_assembly_get,
    PromptAssemblyInput,
    prompt_records_actions::prompt_assembly_get,
);
prompt_web_no_input_command!(
    prompt_assembly_list,
    prompt_records_actions::prompt_assembly_list
);
prompt_web_user_input_unit_command!(
    prompt_assembly_reset,
    PromptAssemblyInput,
    prompt_records_actions::prompt_assembly_reset,
);
prompt_web_user_input_unit_command!(
    prompt_assembly_save,
    SavePromptAssemblyInput,
    prompt_records_actions::prompt_assembly_save,
);
prompt_web_user_input_command!(
    prompt_render,
    RenderInput,
    prompt_records_actions::prompt_render,
);
prompt_web_no_input_command!(prompt_list, prompt_records_actions::prompt_list);
prompt_web_no_input_command!(
    prompt_load_defaults,
    prompt_records_actions::prompt_load_defaults,
);
prompt_web_user_input_command!(
    prompt_update,
    UpdatePromptInput,
    prompt_records_actions::prompt_update,
);
prompt_web_name_unit_command!(prompt_reset, prompt_records_actions::prompt_reset);
prompt_web_name_sync_command!(
    prompt_version_history,
    prompt_records_actions::prompt_version_history,
);
prompt_web_input_command!(
    prompt_version_diff,
    VersionDiffInput,
    prompt_records_actions::prompt_version_diff,
);
prompt_web_input_command!(
    prompt_validate_variables,
    ValidateVarsInput,
    prompt_records_actions::prompt_validate_variables,
);
prompt_web_user_input_command!(
    prompt_render_ab,
    RenderInput,
    prompt_records_actions::prompt_render_ab,
);
prompt_web_user_input_unit_command!(
    prompt_set_project_override,
    SetOverrideInput,
    prompt_records_actions::prompt_set_project_override,
);
prompt_web_user_input_unit_command!(
    prompt_remove_project_override,
    RemoveOverrideInput,
    prompt_records_actions::prompt_remove_project_override,
);
prompt_web_no_input_command!(
    prompt_list_categories,
    prompt_records_actions::prompt_list_categories,
);
