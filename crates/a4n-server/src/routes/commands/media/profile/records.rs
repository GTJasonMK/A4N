use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, command_input, downstream_strategy_arg, i64_arg,
    object_profile_records_actions, scene_profile_records_actions, to_value,
};

macro_rules! profile_record_web_commands {
    (
        $list_fn:ident, $create_fn:ident, $update_fn:ident, $delete_fn:ident,
        $list_input:ty, $input:ty,
        $list_action:path, $create_action:path, $update_action:path, $delete_action:path,
        $id_arg:literal, $id_var:ident $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $list_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $list_input = command_input(args, stringify!($list_fn))?;
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
            let project_id = i64_arg(args, "projectId")?;
            let input: $input = command_input(args, stringify!($create_fn))?;
            to_value(
                $create_action(state.runtime(), user_id, project_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }

        pub(in crate::routes::commands) async fn $update_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let $id_var = i64_arg(args, $id_arg)?;
            let input: $input = command_input(args, stringify!($update_fn))?;
            to_value(
                $update_action(state.runtime(), user_id, $id_var, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }

        pub(in crate::routes::commands) async fn $delete_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let $id_var = i64_arg(args, $id_arg)?;
            let downstream_strategy = downstream_strategy_arg(args)?;
            $delete_action(state.runtime(), user_id, $id_var, downstream_strategy)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

profile_record_web_commands!(
    list_object_profiles,
    create_object_profile,
    update_object_profile,
    delete_object_profile,
    object_profile_records_actions::ListObjectProfilesInput,
    object_profile_records_actions::ObjectProfileInput,
    object_profile_records_actions::list_object_profiles,
    object_profile_records_actions::create_object_profile,
    object_profile_records_actions::update_object_profile,
    object_profile_records_actions::delete_object_profile,
    "objectProfileId",
    object_profile_id,
);

profile_record_web_commands!(
    list_scene_profiles,
    create_scene_profile,
    update_scene_profile,
    delete_scene_profile,
    scene_profile_records_actions::ListSceneProfilesInput,
    scene_profile_records_actions::SceneProfileInput,
    scene_profile_records_actions::list_scene_profiles,
    scene_profile_records_actions::create_scene_profile,
    scene_profile_records_actions::update_scene_profile,
    scene_profile_records_actions::delete_scene_profile,
    "sceneProfileId",
    scene_profile_id,
);
