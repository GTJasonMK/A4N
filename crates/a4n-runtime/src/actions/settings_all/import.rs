mod prompts;
mod records;
mod results;
mod rollback;

use super::types::{AllConfigsExport, ImportAllConfigsInput, ImportAllResult};
use super::validation::{normalize_all_configs_export, validate_all_configs_export};
use crate::AppState;
use a4n_core::error::AppError;
use a4n_import_core::{ImportDocumentKind, ImportNormalizeJsonInput};

pub async fn import_all_configs(
    state: &AppState,
    input: ImportAllConfigsInput,
) -> Result<ImportAllResult, AppError> {
    import_all_configs_inner(state, input)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn import_all_configs_inner(
    state: &AppState,
    input: ImportAllConfigsInput,
) -> Result<ImportAllResult, String> {
    let mut incoming = decode_all_configs_export(state, input.settings_json).await?;
    validate_all_configs_export(&incoming)?;
    normalize_all_configs_export(&mut incoming)?;
    prompts::validate_prompt_import_targets(state, &incoming).await?;

    let config_import = records::settings_all_config_import_input(&incoming, input.overwrite);
    state
        .orchestrator
        .validate_settings_all_config_import_defaults(&config_import)
        .await
        .map_err(results::config_import_error)?;
    let prompt_rows = std::mem::take(&mut incoming.prompts_modified);
    let prompt_snapshots = prompts::prompt_import_snapshots(state, &prompt_rows).await?;
    let prompt_imported = prompts::import_prompt_rows(state, prompt_rows).await?;

    let db_import_result = state
        .orchestrator
        .import_settings_all_configs(config_import)
        .await
        .map(|counts| results::import_result_from_counts(counts, prompt_imported))
        .map_err(results::config_import_error);

    match db_import_result {
        Ok(result) => Ok(result),
        Err(err) => {
            Err(rollback::rollback_prompt_import_after_db_error(state, prompt_snapshots, err).await)
        }
    }
}

async fn decode_all_configs_export(
    state: &AppState,
    settings_json: String,
) -> Result<AllConfigsExport, String> {
    let normalized = crate::sidecar_clients::normalize_import_json(
        state.runtime_profile,
        state.capabilities(),
        ImportNormalizeJsonInput {
            kind: ImportDocumentKind::SettingsAll,
            content: settings_json,
        },
    )
    .await
    .map_err(|err| format!("JSON 解析失败：{err}"))?;

    serde_json::from_value(normalized.normalized).map_err(|err| format!("JSON 解析失败：{err}"))
}

#[cfg(test)]
use self::prompts::{
    import_prompt_rows_for_engine, prompt_import_snapshots_for_engine,
    validate_prompt_import_targets_for_engine,
};
#[cfg(test)]
use self::rollback::rollback_prompt_import_after_db_error_for_engine;

#[cfg(test)]
mod tests;
