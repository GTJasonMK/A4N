use a4n_api_contract::blueprint::{
    BlueprintDiffSummary, BlueprintMutationResult, BlueprintVersionSummary,
    DiffBlueprintVersionsInput, ListBlueprintVersionsInput, RestoreBlueprintVersionInput,
};
use a4n_core::error::AppError;

use crate::AppState;

use super::manual::blueprint_material_sync_warnings;
use super::projections::blueprint_version_summary;

pub async fn list_blueprint_versions(
    state: &AppState,
    user_id: i64,
    input: ListBlueprintVersionsInput,
) -> Result<Vec<BlueprintVersionSummary>, AppError> {
    let versions = state
        .orchestrator()
        .blueprint_versions_for_user_validated(user_id, input.project_id)
        .await?;
    versions.iter().map(blueprint_version_summary).collect()
}

pub async fn diff_blueprint_versions(
    state: &AppState,
    user_id: i64,
    input: DiffBlueprintVersionsInput,
) -> Result<BlueprintDiffSummary, AppError> {
    let diff = state
        .orchestrator()
        .diff_blueprint_versions_for_user_validated(
            user_id,
            input.project_id,
            &input.version_a,
            &input.version_b,
        )
        .await?;
    Ok(BlueprintDiffSummary { diff })
}

pub async fn restore_blueprint_version(
    state: &AppState,
    user_id: i64,
    input: RestoreBlueprintVersionInput,
) -> Result<BlueprintMutationResult, AppError> {
    state
        .orchestrator()
        .restore_blueprint_version_for_user_validated(user_id, input.project_id, &input.version)
        .await?;
    Ok(BlueprintMutationResult {
        warnings: blueprint_material_sync_warnings(state, input.project_id).await,
    })
}
