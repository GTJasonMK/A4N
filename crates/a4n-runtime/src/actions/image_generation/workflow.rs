use super::batch::BatchImageJob;
use super::{GeneratedImageSummary, to_summary};
use crate::AppState;
use crate::actions::workflow_checkpoint::{
    load_completed_workflow_checkpoint, required_checkpoint_field, required_checkpoint_json,
    save_completed_workflow_checkpoint,
};
use crate::file_utils::canonicalize_data_file_path;
use a4n_core::models::{ai::generated_image, generation::workflow_run};
use a4n_services::generation::generation_checkpoint_fingerprint_sha256;
use a4n_services::source::source_content_sha256;
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::EntityTrait;
use serde_json::{Value, json};

pub(super) struct ImageGenerationWorkflowRun {
    pub(super) workflow_run_id: i64,
    pub(super) step_key: String,
    pub(super) fingerprint_json: Value,
}

pub(super) async fn start_batch_image_workflow(
    state: &AppState,
    project_id: i64,
    requests_json: &Value,
) -> Result<workflow_run::Model, String> {
    state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
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

fn batch_image_workflow_business_key(
    project_id: i64,
    requests_json: &Value,
) -> Result<String, String> {
    let serialized = serde_json::to_string(requests_json)
        .map_err(|err| format!("批量图片请求序列化失败: {err}"))?;
    Ok(format!(
        "image_batch:{project_id}:{}",
        source_content_sha256(&serialized)
    ))
}

fn batch_image_workflow_fingerprint(project_id: i64, requests_json: &Value) -> Value {
    json!({
        "workflow": "image_batch",
        "project_id": project_id,
        "requests": requests_json,
    })
}

pub(super) fn batch_image_item_step_key(index: usize) -> String {
    format!("image_batch.item.{}", index + 1)
}

pub(super) fn batch_image_item_fingerprint(
    project_id: i64,
    job: &BatchImageJob,
    request_json: &Value,
) -> Value {
    json!({
        "workflow": "image_batch",
        "project_id": project_id,
        "index": job.index,
        "image_type": job.image_type,
        "chapter_id": job.chapter_id,
        "panel_id": job.panel_id,
        "metadata_json": job.metadata_json,
        "request": request_json,
    })
}

pub(super) async fn start_image_generation_workflow(
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
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
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
            current_step_key: Some(image_generation_step_key(command)),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(ImageGenerationWorkflowRun {
        workflow_run_id: workflow_run.id,
        step_key: image_generation_step_key(command),
        fingerprint_json,
    })
}

pub(super) fn attach_image_generation_workflow(
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

fn image_generation_workflow_fingerprint(
    project_id: i64,
    chapter_id: Option<i64>,
    command: &str,
    image_type: &str,
    panel_id: Option<&str>,
    request_json: &Value,
    metadata_json: Option<&Value>,
) -> Value {
    json!({
        "workflow": "image_generation",
        "project_id": project_id,
        "chapter_id": chapter_id,
        "command": command,
        "image_type": image_type,
        "panel_id": panel_id,
        "request": request_json,
        "metadata": metadata_json,
    })
}

fn image_generation_step_key(command: &str) -> String {
    format!("image_generation.{command}")
}

pub(super) async fn load_completed_image_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    step_key: &str,
    item_fingerprint: &Value,
    project_id: i64,
) -> Result<Option<GeneratedImageSummary>, String> {
    let Some(checkpoint) = load_completed_workflow_checkpoint(
        state,
        Some(workflow_run_id),
        step_key,
        item_fingerprint,
    )
    .await?
    else {
        return Ok(None);
    };
    let checkpoint_json = required_checkpoint_json(&checkpoint, step_key)?;
    let image_id = required_checkpoint_field(&checkpoint_json, step_key, "generated_image_id")?
        .as_i64()
        .ok_or_else(|| format!("{step_key}.generated_image_id 必须是整数"))?;
    let checkpoint_file_path = required_checkpoint_field(&checkpoint_json, step_key, "file_path")?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| format!("{step_key}.file_path 必须是字符串"))?;
    let image = generated_image::Entity::find_by_id(image_id)
        .one(state.db.as_ref())
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

pub(super) async fn save_completed_image_checkpoint(
    state: &AppState,
    workflow: &ImageGenerationWorkflowRun,
    image: &generated_image::Model,
    summary: &GeneratedImageSummary,
) -> Result<(), String> {
    save_completed_workflow_checkpoint(
        state,
        Some(workflow.workflow_run_id),
        workflow.step_key.clone(),
        1,
        workflow.fingerprint_json.clone(),
        json!({
            "generated_image_id": image.id,
            "file_path": image.file_path,
            "summary": summary,
        }),
    )
    .await
}

pub(super) async fn save_completed_batch_image_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    step_key: String,
    order: i32,
    item_fingerprint: Value,
    image: &generated_image::Model,
    summary: &GeneratedImageSummary,
) -> Result<(), String> {
    save_completed_workflow_checkpoint(
        state,
        Some(workflow_run_id),
        step_key,
        order,
        item_fingerprint,
        json!({
            "generated_image_id": image.id,
            "file_path": image.file_path,
            "summary": summary,
        }),
    )
    .await
}

pub(super) async fn finalize_single_image_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<GeneratedImageSummary, String>,
) -> Result<(), String> {
    finalize_image_workflow(
        state,
        workflow_run_id,
        result.as_ref().map(|_| ()).map_err(String::as_str),
    )
    .await
}

pub(super) async fn finalize_batch_image_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<Vec<GeneratedImageSummary>, String>,
) -> Result<(), String> {
    finalize_image_workflow(
        state,
        workflow_run_id,
        result.as_ref().map(|_| ()).map_err(String::as_str),
    )
    .await
}

async fn finalize_image_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: Result<(), &str>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message)),
    };
    state
        .workflow_run
        .mark_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
