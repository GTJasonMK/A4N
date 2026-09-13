use super::super::{
    ApiError, blueprint_records_actions, i64_arg, reject_unknown_command_args,
    strict_command_input, strict_runtime_command_input, to_value,
};
use crate::state::ServerState;
use blueprint_records_actions::{PreviewBlueprintGenerationInput, PreviewBlueprintRefinementInput};
use serde_json::Value;

pub(in crate::routes::commands) async fn get_blueprint(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    reject_unknown_command_args(args, "get_blueprint", &["projectId"])?;
    let project_id = i64_arg(args, "projectId")?;
    to_value(blueprint_records_actions::get_blueprint(state.runtime(), user_id, project_id).await?)
}

pub(in crate::routes::commands) async fn sync_blueprint_materials(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    reject_unknown_command_args(
        args,
        "sync_blueprint_materials",
        &["projectId", "runtime_config_snapshot"],
    )?;
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        blueprint_records_actions::sync_blueprint_materials(state.runtime(), user_id, project_id)
            .await?,
    )
}

macro_rules! blueprint_web_input_command {
    ($name:ident, $input:ty, $parse:path, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = $parse(args, stringify!($name))?;
            to_value($action(state.runtime(), user_id, input).await?)
        }
    };
}

blueprint_web_input_command!(
    preview_generate_blueprint_request,
    PreviewBlueprintGenerationInput,
    strict_runtime_command_input,
    blueprint_records_actions::preview_generate_blueprint_request,
);

blueprint_web_input_command!(
    preview_refine_blueprint_request,
    PreviewBlueprintRefinementInput,
    strict_runtime_command_input,
    blueprint_records_actions::preview_refine_blueprint_request,
);

blueprint_web_input_command!(
    list_blueprint_versions,
    blueprint_records_actions::ListBlueprintVersionsInput,
    strict_command_input,
    blueprint_records_actions::list_blueprint_versions,
);

blueprint_web_input_command!(
    diff_blueprint_versions,
    blueprint_records_actions::DiffBlueprintVersionsInput,
    strict_command_input,
    blueprint_records_actions::diff_blueprint_versions,
);

blueprint_web_input_command!(
    restore_blueprint_version,
    blueprint_records_actions::RestoreBlueprintVersionInput,
    strict_runtime_command_input,
    blueprint_records_actions::restore_blueprint_version,
);

blueprint_web_input_command!(
    save_blueprint,
    blueprint_records_actions::SaveBlueprintInput,
    strict_runtime_command_input,
    blueprint_records_actions::save_blueprint,
);

blueprint_web_input_command!(
    update_world_setting,
    blueprint_records_actions::UpdateWorldSettingInput,
    strict_runtime_command_input,
    blueprint_records_actions::update_world_setting,
);
