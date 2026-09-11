use super::GalgameProjectInput;
use crate::AppState;
use a4n_api_contract::export::ExportFileData;

pub async fn export_galgame_webgal(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<ExportFileData, String> {
    state
        .export_application()
        .export_galgame_webgal(user_id, input.project_id)
        .await
        .map_err(crate::actions::app_error_message)
}
