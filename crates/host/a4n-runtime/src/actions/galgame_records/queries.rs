use super::{GalgameAssetPlan, GalgameProjectInput, GalgameScriptDetail};
use crate::AppState;

pub async fn get_galgame_script(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<Option<GalgameScriptDetail>, String> {
    state
        .galgame_read_application()
        .script(user_id, input.project_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn get_galgame_asset_plan(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<Option<GalgameAssetPlan>, String> {
    state
        .galgame_read_application()
        .asset_plan(user_id, input.project_id)
        .await
        .map_err(|err| err.to_string())
}
