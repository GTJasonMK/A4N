use super::super::{GalgameProjectInput, load_galgame_script_detail};
use super::package::build_webgal_package;
use super::rules::sanitize_export_file_stem;
use crate::AppState;
use crate::actions::access;
use crate::actions::export_files::ExportFileData;
use base64::Engine as _;

pub async fn export_galgame_webgal(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<ExportFileData, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let detail = load_galgame_script_detail(state, input.project_id).await?;
    if detail.scenes.is_empty() {
        return Err("Galgame 剧本没有可导出的场".to_string());
    }
    let portraits = state
        .orchestrator
        .galgame_webgal_portraits(detail.script.root_project_id)
        .await
        .map_err(|err| {
            format!(
                "查询 Galgame 立绘失败: {}",
                crate::actions::app_error_message(err)
            )
        })?;
    let bytes = build_webgal_package(&state.config.data_dir, &detail, &portraits)?;
    Ok(ExportFileData {
        file_name: format!(
            "{}-webgal.zip",
            sanitize_export_file_stem(&detail.script.title)
        ),
        mime_type: "application/zip".to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}
