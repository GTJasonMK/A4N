use super::{GalgameProjectInput, GalgameScriptDetail};
use crate::AppState;

pub async fn get_galgame_script(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameScriptDetail, String> {
    state
        .orchestrator()
        .get_galgame_script_for_user_validated(user_id, input.project_id)
        .await
        .map_err(|err| err.to_string())
}
