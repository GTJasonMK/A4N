use a4n_core::error::AppError;

use crate::AppState;

use super::contracts::{
    BlueprintDiffSummary, BlueprintSummary, BlueprintVersionSummary, DiffBlueprintVersionsInput,
    RestoreBlueprintVersionInput,
};
use super::projections::{blueprint_summary, blueprint_version_summary};

pub async fn list_blueprint_versions(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<BlueprintVersionSummary>, AppError> {
    let versions = state
        .project_blueprint()
        .blueprint_versions_for_user_validated(user_id, project_id)
        .await?;
    Ok(versions.iter().map(blueprint_version_summary).collect())
}

pub async fn diff_blueprint_versions(
    state: &AppState,
    user_id: i64,
    input: DiffBlueprintVersionsInput,
) -> Result<BlueprintDiffSummary, AppError> {
    let diff = state
        .project_blueprint()
        .diff_blueprint_versions_for_user_validated(
            user_id,
            input.project_id,
            &input.version_a,
            &input.version_b,
        )
        .await?;
    Ok(BlueprintDiffSummary {
        version_a: input.version_a,
        version_b: input.version_b,
        diff,
    })
}

pub async fn restore_blueprint_version(
    state: &AppState,
    user_id: i64,
    input: RestoreBlueprintVersionInput,
) -> Result<BlueprintSummary, AppError> {
    let result = state
        .project_blueprint()
        .restore_blueprint_version_for_user_validated(user_id, input.project_id, &input.version)
        .await?;

    Ok(blueprint_summary(&result.blueprint))
}
