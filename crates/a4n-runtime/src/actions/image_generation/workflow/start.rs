use super::ImageGenerationWorkflowRun;
use super::fingerprints::{
    batch_image_workflow_business_key, batch_image_workflow_fingerprint, image_generation_step_key,
    image_generation_workflow_fingerprint,
};
use crate::AppState;
use a4n_core::models::generation::workflow_run;
use a4n_orchestrator::{
    generation::generation_checkpoint_fingerprint_sha256,
    workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING},
};
use serde_json::Value;

pub(crate) async fn start_batch_image_workflow(
    state: &AppState,
    project_id: i64,
    requests_json: &Value,
) -> Result<workflow_run::Model, String> {
    state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: batch_image_workflow_business_key(project_id, requests_json)?,
            workflow_kind: "image_batch".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some("batch".to_string()),
            input_fingerprint_json: Some(batch_image_workflow_fingerprint(
                project_id,
                requests_json,
            )),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("image_batch.prepare".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn start_image_generation_workflow(
    state: &AppState,
    project_id: i64,
    chapter_id: Option<i64>,
    command: &str,
    image_type: &str,
    panel_id: Option<&str>,
    request_json: &Value,
    metadata_json: Option<&Value>,
) -> Result<ImageGenerationWorkflowRun, String> {
    let fingerprint_json = image_generation_workflow_fingerprint(
        project_id,
        chapter_id,
        command,
        image_type,
        panel_id,
        request_json,
        metadata_json,
    );
    let fingerprint_sha = generation_checkpoint_fingerprint_sha256(&fingerprint_json)
        .map_err(|err| err.to_string())?;
    let step_key = image_generation_step_key(command);
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!("image_generation:{project_id}:{command}:{fingerprint_sha}"),
            workflow_kind: "image_generation".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(project_id),
            chapter_id,
            mode: Some(image_type.to_string()),
            input_fingerprint_json: Some(fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(step_key.clone()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(ImageGenerationWorkflowRun {
        workflow_run_id: workflow_run.id,
        step_key,
        fingerprint_json,
    })
}

pub(crate) fn attach_image_generation_workflow(
    workflow_run_id: i64,
    project_id: i64,
    chapter_id: Option<i64>,
    command: &str,
    image_type: &str,
    panel_id: Option<&str>,
    request_json: &Value,
    metadata_json: Option<&Value>,
) -> Result<ImageGenerationWorkflowRun, String> {
    if workflow_run_id <= 0 {
        return Err("图片生成 workflow_run_id 必须是正整数".to_string());
    }
    Ok(ImageGenerationWorkflowRun {
        workflow_run_id,
        step_key: image_generation_step_key(command),
        fingerprint_json: image_generation_workflow_fingerprint(
            project_id,
            chapter_id,
            command,
            image_type,
            panel_id,
            request_json,
            metadata_json,
        ),
    })
}
