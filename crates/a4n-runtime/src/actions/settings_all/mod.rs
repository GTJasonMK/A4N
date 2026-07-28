mod host_port;

use a4n_core::error::AppError;
pub use a4n_orchestrator::config::{ImportAllConfigsInput, ImportAllResult};

use crate::AppState;

use self::host_port::RuntimeSettingsAllHostPort;

pub async fn export_all_configs_to_json(state: &AppState) -> Result<String, AppError> {
    state
        .settings()
        .export_all_configs_to_json_validated(&RuntimeSettingsAllHostPort { state })
        .await
}

pub async fn import_all_configs(
    state: &AppState,
    input: ImportAllConfigsInput,
) -> Result<ImportAllResult, AppError> {
    state
        .settings()
        .import_all_configs_validated(&RuntimeSettingsAllHostPort { state }, input)
        .await
}
