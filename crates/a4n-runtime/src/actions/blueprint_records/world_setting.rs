use crate::AppState;
use a4n_api_contract::blueprint::{BlueprintMutationResult, UpdateWorldSettingInput};
use a4n_error::AppError;

use super::manual::blueprint_material_sync_warnings;

pub async fn update_world_setting(
    state: &AppState,
    user_id: i64,
    input: UpdateWorldSettingInput,
) -> Result<BlueprintMutationResult, AppError> {
    let project_id = input.project_id;
    state
        .blueprint_mutation_application()
        .update_world_setting(user_id, input)
        .await?;
    Ok(BlueprintMutationResult {
        warnings: blueprint_material_sync_warnings(state, project_id).await,
    })
}
