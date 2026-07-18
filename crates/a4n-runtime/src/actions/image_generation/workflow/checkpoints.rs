use super::super::{GeneratedImageSummary, to_summary};
use super::ImageGenerationWorkflowRun;
use crate::{AppState, file_utils::canonicalize_data_file_path};
use a4n_core::models::ai::generated_image;
use a4n_orchestrator::workflow::{
    CompletedWorkflowCheckpointInput, required_checkpoint_field, required_checkpoint_json,
};
use serde_json::{Value, json};

pub(crate) async fn load_completed_image_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    step_key: &str,
    item_fingerprint: &Value,
    project_id: i64,
) -> Result<Option<GeneratedImageSummary>, String> {
    let Some(checkpoint) = state
        .orchestrator
        .load_completed_workflow_checkpoint(Some(workflow_run_id), step_key, item_fingerprint)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json =
        required_checkpoint_json(&checkpoint, step_key).map_err(|error| error.to_string())?;
    let image_id = required_checkpoint_field(&checkpoint_json, step_key, "generated_image_id")
        .map_err(|error| error.to_string())?
        .as_i64()
        .ok_or_else(|| format!("{step_key}.generated_image_id 必须是整数"))?;
    let checkpoint_file_path = required_checkpoint_field(&checkpoint_json, step_key, "file_path")
        .map_err(|error| error.to_string())?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| format!("{step_key}.file_path 必须是字符串"))?;
    let image = state
        .orchestrator
        .generated_image_by_id(image_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("{step_key} checkpoint 指向的图片记录不存在: {image_id}"))?;
    if image.project_id != project_id {
        return Err(format!(
            "{step_key} checkpoint 指向其他项目图片: image_project_id={}",
            image.project_id
        ));
    }
    if image.file_path != checkpoint_file_path {
        return Err(format!(
            "{step_key} checkpoint 图片路径不一致: checkpoint={}, db={}",
            checkpoint_file_path, image.file_path
        ));
    }
    canonicalize_data_file_path(&state.config.data_dir, &image.file_path, "断点图片文件")?;
    Ok(Some(to_summary(&image)))
}

pub(crate) async fn save_completed_image_checkpoint(
    state: &AppState,
    workflow: &ImageGenerationWorkflowRun,
    image: &generated_image::Model,
    summary: &GeneratedImageSummary,
) -> Result<(), String> {
    state
        .orchestrator
        .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(workflow.workflow_run_id),
            step_key: workflow.step_key.clone(),
            step_order: 1,
            input_fingerprint_json: workflow.fingerprint_json.clone(),
            checkpoint_json: json!({
                "generated_image_id": image.id,
                "file_path": image.file_path,
                "summary": summary,
            }),
        })
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn save_completed_batch_image_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    step_key: String,
    order: i32,
    item_fingerprint: Value,
    image: &generated_image::Model,
    summary: &GeneratedImageSummary,
) -> Result<(), String> {
    state
        .orchestrator
        .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(workflow_run_id),
            step_key,
            step_order: order,
            input_fingerprint_json: item_fingerprint,
            checkpoint_json: json!({
                "generated_image_id": image.id,
                "file_path": image.file_path,
                "summary": summary,
            }),
        })
        .await
        .map_err(|error| error.to_string())
}
