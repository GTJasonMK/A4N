use super::input::{empty_command_args, strict_command_input, to_value};
use super::{ApiError, ServerState};
use a4n_api_contract::package::{
    GetPackageOperationInput, ListPackageCommandAvailabilityInput, StartPackageLifecycleInput,
    UpdatePackagePreferencesInput,
};
use a4n_runtime::host_api::package as package_actions;
use serde_json::Value;

pub(in crate::routes::commands) async fn list_package_command_availability(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: ListPackageCommandAvailabilityInput =
        strict_command_input(args, "list_package_command_availability")?;
    to_value(
        package_actions::list_package_command_availability(state.runtime(), user_id, input).await?,
    )
}

pub(in crate::routes::commands) async fn get_package_overview(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "get_package_overview")?;
    to_value(package_actions::get_package_overview(state.runtime()).await?)
}

pub(in crate::routes::commands) async fn get_package_operation(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: GetPackageOperationInput = strict_command_input(args, "get_package_operation")?;
    to_value(package_actions::get_package_operation(state.runtime(), user_id, input).await?)
}

pub(in crate::routes::commands) async fn start_package_lifecycle(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: StartPackageLifecycleInput = strict_command_input(args, "start_package_lifecycle")?;
    to_value(package_actions::start_package_lifecycle(state.runtime(), user_id, input).await?)
}

pub(in crate::routes::commands) async fn get_package_preferences(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "get_package_preferences")?;
    to_value(package_actions::get_package_preferences(state.runtime(), user_id).await?)
}

pub(in crate::routes::commands) async fn update_package_preferences(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: UpdatePackagePreferencesInput =
        strict_command_input(args, "update_package_preferences")?;
    to_value(package_actions::update_package_preferences(state.runtime(), user_id, input).await?)
}
