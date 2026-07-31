use super::GalgameProjectInput;
use crate::AppState;
use crate::file_utils::canonicalize_data_file_path;
use a4n_orchestrator::export::ExportFileData;
use a4n_orchestrator::galgame::WebgalExportHostPort;

pub async fn export_galgame_webgal(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<ExportFileData, String> {
    let host_port = RuntimeWebgalExportHostPort { state };
    state
        .galgame()
        .run_export_galgame_webgal_for_user_validated(&host_port, user_id, input.project_id)
        .await
        .map_err(crate::actions::app_error_message)
}

struct RuntimeWebgalExportHostPort<'a> {
    state: &'a AppState,
}

impl WebgalExportHostPort for RuntimeWebgalExportHostPort<'_> {
    fn read_data_file(&self, relative_path: &str, label: &str) -> Result<Vec<u8>, String> {
        let path = canonicalize_data_file_path(self.state.data_dir(), relative_path, label)?;
        std::fs::read(&path)
            .map_err(|error| format!("读取{label}失败: {}: {error}", path.display()))
    }
}
