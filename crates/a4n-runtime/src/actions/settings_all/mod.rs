mod host_port;

use a4n_api_contract::config::ImportAllConfigsInput;
use a4n_core::error::AppError;

use crate::{
    AppState,
    actions::admin_records::{AdminPermission, require_admin_permission},
};

use self::host_port::RuntimeSettingsAllHostPort;

pub async fn export_all_configs_to_json(
    state: &AppState,
    caller_user_id: i64,
) -> Result<String, AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await?;
    state
        .orchestrator()
        .export_all_configs_to_json_validated(&RuntimeSettingsAllHostPort { state })
        .await
}

pub async fn import_all_configs(
    state: &AppState,
    caller_user_id: i64,
    input: ImportAllConfigsInput,
) -> Result<Vec<String>, AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await?;
    state
        .orchestrator()
        .import_all_configs_validated(&RuntimeSettingsAllHostPort { state }, input)
        .await
}
