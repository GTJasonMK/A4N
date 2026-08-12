use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, command_input, foreshadowing_records_actions,
    object_profile_records_actions, scene_profile_records_actions, to_value,
};
use super::super::super::{strict_command_input, strict_runtime_command_input};

macro_rules! profile_record_web_commands {
    (
        $list_fn:ident, $create_fn:ident, $update_fn:ident, $delete_fn:ident,
        $list_input:ty, $create_input:ty, $update_input:ty, $delete_input:ty,
        $create_input_parser:path, $update_input_parser:path, $delete_input_parser:path,
        $list_action:path, $create_action:path, $update_action:path, $delete_action:path,
        $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $list_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $list_input = strict_command_input(args, stringify!($list_fn))?;
            to_value(
                $list_action(state.runtime(), user_id, input.project_id)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }

        pub(in crate::routes::commands) async fn $create_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $create_input = $create_input_parser(args, stringify!($create_fn))?;
            to_value(
                $create_action(state.runtime(), user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }

        pub(in crate::routes::commands) async fn $update_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $update_input = $update_input_parser(args, stringify!($update_fn))?;
            to_value(
                $update_action(state.runtime(), user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }

        pub(in crate::routes::commands) async fn $delete_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $delete_input = $delete_input_parser(args, stringify!($delete_fn))?;
            to_value(
                $delete_action(state.runtime(), user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

profile_record_web_commands!(
    list_object_profiles,
    create_object_profile,
    update_object_profile,
    delete_object_profile,
    object_profile_records_actions::ListObjectProfilesInput,
    object_profile_records_actions::CreateObjectProfileInput,
    object_profile_records_actions::UpdateObjectProfileInput,
    object_profile_records_actions::DeleteObjectProfileInput,
    strict_command_input,
    strict_command_input,
    strict_command_input,
    object_profile_records_actions::list_object_profiles,
    object_profile_records_actions::create_object_profile,
    object_profile_records_actions::update_object_profile,
    object_profile_records_actions::delete_object_profile,
);

profile_record_web_commands!(
    list_scene_profiles,
    create_scene_profile,
    update_scene_profile,
    delete_scene_profile,
    scene_profile_records_actions::ListSceneProfilesInput,
    scene_profile_records_actions::CreateSceneProfileInput,
    scene_profile_records_actions::UpdateSceneProfileInput,
    scene_profile_records_actions::DeleteSceneProfileInput,
    strict_runtime_command_input,
    strict_runtime_command_input,
    strict_runtime_command_input,
    scene_profile_records_actions::list_scene_profiles,
    scene_profile_records_actions::create_scene_profile,
    scene_profile_records_actions::update_scene_profile,
    scene_profile_records_actions::delete_scene_profile,
);

pub(in crate::routes::commands) async fn list_foreshadowing(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: foreshadowing_records_actions::ListForeshadowingInput =
        command_input(args, "list_foreshadowing")?;
    to_value(
        foreshadowing_records_actions::list_foreshadowing(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn create_foreshadowing(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: foreshadowing_records_actions::CreateForeshadowingInput =
        command_input(args, "create_foreshadowing")?;
    to_value(
        foreshadowing_records_actions::create_foreshadowing(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn update_foreshadowing(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: foreshadowing_records_actions::UpdateForeshadowingInput =
        command_input(args, "update_foreshadowing")?;
    to_value(
        foreshadowing_records_actions::update_foreshadowing(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn delete_foreshadowing(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: foreshadowing_records_actions::DeleteForeshadowingInput =
        command_input(args, "delete_foreshadowing")?;
    to_value(
        foreshadowing_records_actions::delete_foreshadowing(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?,
    )
}
