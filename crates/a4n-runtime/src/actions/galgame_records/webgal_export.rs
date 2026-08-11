use super::GalgameProjectInput;
use crate::AppState;
use a4n_api_contract::export::ExportFileData;

pub async fn export_galgame_webgal(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<ExportFileData, String> {
    state
        .orchestrator()
        .run_export_galgame_webgal_for_user_validated(user_id, input.project_id)
        .await
        .map_err(crate::actions::app_error_message)
}
