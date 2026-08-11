use crate::AppState;
use a4n_core::error::AppError;

use super::contracts::{BlueprintMutationResult, UpdateWorldSettingInput};
use super::manual::blueprint_material_sync_warnings;

pub async fn update_world_setting(
    state: &AppState,
    user_id: i64,
    input: UpdateWorldSettingInput,
) -> Result<BlueprintMutationResult, AppError> {
    let project_id = input.project_id;
    state
        .orchestrator()
        .update_blueprint_world_setting_for_user_validated(user_id, project_id, input.world_setting)
        .await?;
    Ok(BlueprintMutationResult {
        warnings: blueprint_material_sync_warnings(state, project_id).await,
    })
}
