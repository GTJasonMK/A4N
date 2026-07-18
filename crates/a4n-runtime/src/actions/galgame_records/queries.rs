use crate::{AppState, actions::access};
use a4n_core::models::project::novel_project;
use a4n_orchestrator::galgame::GalgameScriptDetail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalgameProjectInput {
    pub project_id: i64,
}

pub(crate) async fn project_record_by_id(
    state: &AppState,
    project_id: i64,
) -> Result<novel_project::Model, String> {
    state
        .orchestrator
        .project_by_id(project_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
}

pub(crate) async fn load_galgame_script_detail(
    state: &AppState,
    project_id: i64,
) -> Result<GalgameScriptDetail, String> {
    state
        .orchestrator
        .get_script_detail(project_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn get_galgame_script(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameScriptDetail, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    load_galgame_script_detail(state, input.project_id).await
}
