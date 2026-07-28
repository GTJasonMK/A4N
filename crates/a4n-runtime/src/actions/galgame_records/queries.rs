use crate::AppState;
use a4n_orchestrator::galgame::GalgameScriptDetail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalgameProjectInput {
    pub project_id: i64,
}

pub async fn get_galgame_script(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameScriptDetail, String> {
    state
        .galgame()
        .get_galgame_script_for_user_validated(user_id, input.project_id)
        .await
        .map_err(|err| err.to_string())
}
