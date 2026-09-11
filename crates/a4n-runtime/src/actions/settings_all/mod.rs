mod host_port;

use a4n_api_contract::config::ImportAllConfigsInput;
use a4n_error::AppError;

use crate::AppState;

use self::host_port::RuntimeSettingsAllHostPort;

pub async fn export_all_configs_to_json(
    state: &AppState,
    caller_user_id: i64,
) -> Result<String, AppError> {
    state
        .settings_all_application()
        .export_all_configs_to_json(caller_user_id, &RuntimeSettingsAllHostPort { state })
        .await
}

pub async fn import_all_configs(
    state: &AppState,
    caller_user_id: i64,
    input: ImportAllConfigsInput,
) -> Result<Vec<String>, AppError> {
    state
        .settings_all_application()
        .import_all_configs(caller_user_id, &RuntimeSettingsAllHostPort { state }, input)
        .await
}
