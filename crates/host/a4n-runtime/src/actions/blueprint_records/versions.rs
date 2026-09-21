use a4n_api_contract::blueprint::{
    BlueprintDiffSummary, BlueprintMutationResult, BlueprintVersionSummary,
    DiffBlueprintVersionsInput, ListBlueprintVersionsInput, RestoreBlueprintVersionInput,
};
use a4n_error::AppError;

use crate::AppState;

use super::manual::blueprint_material_sync_warnings;
pub async fn list_blueprint_versions(
    state: &AppState,
    user_id: i64,
    input: ListBlueprintVersionsInput,
) -> Result<Vec<BlueprintVersionSummary>, AppError> {
    state
        .blueprint_read_application()
        .versions(user_id, input)
        .await
}

pub async fn diff_blueprint_versions(
    state: &AppState,
    user_id: i64,
    input: DiffBlueprintVersionsInput,
) -> Result<BlueprintDiffSummary, AppError> {
    state
        .blueprint_read_application()
        .diff(user_id, input)
        .await
}

pub async fn restore_blueprint_version(
    state: &AppState,
    user_id: i64,
    input: RestoreBlueprintVersionInput,
) -> Result<BlueprintMutationResult, AppError> {
    let project_id = input.project_id;
    state
        .blueprint_mutation_application()
        .restore(user_id, input)
        .await?;
    Ok(BlueprintMutationResult {
        warnings: blueprint_material_sync_warnings(state, project_id).await,
    })
}
