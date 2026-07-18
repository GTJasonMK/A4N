use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, command_input, profile_materials_actions, to_value,
};

pub(in crate::routes::commands) async fn list_profile_materials(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: profile_materials_actions::ListProfileMaterialsInput =
        command_input(args, "list_profile_materials")?;
    let groups = profile_materials_actions::list_profile_materials(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(groups)
}

pub(in crate::routes::commands) async fn set_profile_primary_material(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: profile_materials_actions::SetProfilePrimaryMaterialInput =
        command_input(args, "set_profile_primary_material")?;
    let group =
        profile_materials_actions::set_profile_primary_material(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(group)
}

pub(in crate::routes::commands) async fn upload_profile_material_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: profile_materials_actions::UploadProfileMaterialImageInput =
        command_input(args, "upload_profile_material_image")?;
    let group =
        profile_materials_actions::upload_profile_material_image(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(group)
}
