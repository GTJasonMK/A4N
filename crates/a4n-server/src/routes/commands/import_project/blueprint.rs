use super::super::{ApiError, blueprint_records_actions, command_input, i64_arg, to_value};
use crate::state::ServerState;
use blueprint_records_actions::{PreviewBlueprintGenerationInput, PreviewBlueprintRefinementInput};
use serde_json::Value;

pub(in crate::routes::commands) async fn get_blueprint(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(blueprint_records_actions::get_blueprint(state.runtime(), user_id, project_id).await?)
}

macro_rules! blueprint_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state.runtime(), user_id, input).await?)
        }
    };
}

macro_rules! blueprint_web_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state.runtime(), user_id, input).await?)
        }
    };
}

blueprint_web_preview_command!(
    preview_generate_blueprint_request,
    PreviewBlueprintGenerationInput,
    blueprint_records_actions::preview_generate_blueprint_request,
);

blueprint_web_preview_command!(
    preview_refine_blueprint_request,
    PreviewBlueprintRefinementInput,
    blueprint_records_actions::preview_refine_blueprint_request,
);

pub(in crate::routes::commands) async fn list_blueprint_versions(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        blueprint_records_actions::list_blueprint_versions(state.runtime(), user_id, project_id)
            .await?,
    )
}

blueprint_web_input_command!(
    diff_blueprint_versions,
    blueprint_records_actions::DiffBlueprintVersionsInput,
    blueprint_records_actions::diff_blueprint_versions,
);

blueprint_web_input_command!(
    restore_blueprint_version,
    blueprint_records_actions::RestoreBlueprintVersionInput,
    blueprint_records_actions::restore_blueprint_version,
);

blueprint_web_input_command!(
    update_world_setting,
    blueprint_records_actions::UpdateWorldSettingInput,
    blueprint_records_actions::update_world_setting,
);
