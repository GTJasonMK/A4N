use super::super::{PortraitSummary, character_portrait_by_id, to_summary};
use crate::AppState;
use crate::file_utils::canonicalize_data_file_path;
use a4n_orchestrator::workflow::{required_checkpoint_field, required_checkpoint_json};
use serde_json::Value;

pub(crate) async fn load_completed_portrait_final_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    step_key: &str,
    fingerprint: &Value,
    project_id: i64,
) -> Result<Option<PortraitSummary>, String> {
    let Some(checkpoint) = state
        .orchestrator
        .load_completed_workflow_checkpoint(Some(workflow_run_id), step_key, fingerprint)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json =
        required_checkpoint_json(&checkpoint, step_key).map_err(|error| error.to_string())?;
    let portrait_id = required_checkpoint_field(&checkpoint_json, step_key, "portrait_id")
        .map_err(|error| error.to_string())?
        .as_i64()
        .ok_or_else(|| format!("{step_key}.portrait_id 必须是整数"))?;
    let image_path = portrait_checkpoint_string(&checkpoint_json, step_key, "image_path")?;
    let portrait = character_portrait_by_id(state, portrait_id).await?;
    if portrait.project_id != project_id {
        return Err(format!(
            "{step_key} checkpoint 指向其他项目立绘: portrait_project_id={}",
            portrait.project_id
        ));
    }
    if portrait.image_path != image_path {
        return Err(format!(
            "{step_key} checkpoint 立绘路径不一致: checkpoint={}, db={}",
            image_path, portrait.image_path
        ));
    }
    canonicalize_data_file_path(&state.config.data_dir, &portrait.image_path, "断点立绘图片")?;
    Ok(Some(to_summary(&portrait)))
}

pub(crate) fn portrait_checkpoint_string(
    checkpoint_json: &Value,
    step_key: &str,
    field: &str,
) -> Result<String, String> {
    required_checkpoint_field(checkpoint_json, step_key, field)
        .map_err(|error| error.to_string())?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| format!("{step_key}.{field} 必须是字符串"))
}
