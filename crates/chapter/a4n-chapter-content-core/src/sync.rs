use a4n_db::workflow::{
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WorkflowCheckpointSummary,
    WorkflowRunSummary,
};
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{CHAPTER_CONTENT_SYNC_WORKFLOW_KIND, cleanup_chapter_derived_data_in};

pub const CHAPTER_CONTENT_SYNC_CLEANUP_STEP: &str = "chapter_content_sync.cleanup_plan";
pub const CHAPTER_CONTENT_SYNC_EXTERNAL_STEP: &str = "chapter_content_sync.external_cleanup";

pub async fn begin_chapter_content_sync_in_transaction<C: ConnectionTrait>(
    tx: &C,
    chapter: &ChapterRecord,
) -> Result<WorkflowRunSummary, AppError> {
    let business_key = chapter_content_sync_workflow_business_key(chapter.project_id, chapter.id);
    let previous =
        a4n_db::workflow::latest_workflow_run_summary_by_business_key_for_update(tx, &business_key)
            .await
            .map_err(workflow_read_error)?;
    let mut artifacts = cleanup_chapter_derived_data_in(
        tx,
        chapter.project_id,
        chapter.id,
        chapter.chapter_number,
        None,
    )
    .await?;
    if let Some(previous) = previous.filter(|run| {
        run.workflow_kind == CHAPTER_CONTENT_SYNC_WORKFLOW_KIND
            && a4n_db::workflow::workflow_run_is_running_or_resumable(&run.status, run.can_resume)
    }) {
        let previous_id = previous.id;
        let checkpoint = a4n_db::workflow::workflow_checkpoint_summary_by_step_in(
            tx,
            previous_id,
            CHAPTER_CONTENT_SYNC_CLEANUP_STEP,
        )
        .await
        .map_err(workflow_read_error)?
        .ok_or_else(|| AppError::InvalidState {
            current: format!("chapter_content_sync_run_id={previous_id}"),
            operation: "沿用尚未完成的章节资料清理计划".to_string(),
        })?;
        artifacts.merge(chapter_content_sync_checkpoint_artifacts(
            &previous,
            &checkpoint,
        )?);
    }

    let input = chapter_content_sync_workflow_run_input(chapter);
    let run_fingerprint_json =
        input
            .input_fingerprint_json
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_chapter_content_sync_fingerprint".to_string(),
                operation: "登记章节资料同步".to_string(),
            })?;
    let run = a4n_db::workflow::upsert_workflow_run_summary_in_transaction(tx, input)
        .await
        .map_err(workflow_write_error)?;
    let checkpoint =
        chapter_content_sync_checkpoint_input(run.id, run_fingerprint_json, chapter, &artifacts)?;
    a4n_db::workflow::upsert_workflow_checkpoint_in_transaction(tx, checkpoint)
        .await
        .map_err(workflow_write_error)?;
    Ok(run)
}

pub fn chapter_content_sync_workflow_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("chapter_content_sync:{project_id}:{chapter_id}")
}

pub fn chapter_content_sync_workflow_run_input(chapter: &ChapterRecord) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: chapter_content_sync_workflow_business_key(chapter.project_id, chapter.id),
        workflow_kind: CHAPTER_CONTENT_SYNC_WORKFLOW_KIND.to_string(),
        project_id: Some(chapter.project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("chapter".to_string()),
        scope_id: Some(chapter.id),
        chapter_id: Some(chapter.id),
        mode: Some("content_change".to_string()),
        input_fingerprint_json: Some(chapter_content_sync_run_fingerprint(chapter)),
        status: a4n_domain::workflow::WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(CHAPTER_CONTENT_SYNC_EXTERNAL_STEP.to_string()),
        active_task_id: None,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    }
}

fn chapter_content_sync_run_fingerprint(chapter: &ChapterRecord) -> Value {
    json!({
        "schema_version": 3,
        "workflow_kind": CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
        "sync_revision": Uuid::new_v4(),
        "source_fingerprint": crate::chapter_content_sync_workflow_fingerprint(chapter),
    })
}

pub fn chapter_content_sync_checkpoint_input(
    workflow_run_id: i64,
    run_fingerprint_json: Value,
    chapter: &ChapterRecord,
    artifacts: &ChapterDerivedCleanupArtifacts,
) -> Result<UpsertWorkflowCheckpointInput, AppError> {
    Ok(UpsertWorkflowCheckpointInput {
        workflow_run_id,
        step_key: CHAPTER_CONTENT_SYNC_CLEANUP_STEP.to_string(),
        step_order: 10,
        status: a4n_domain::workflow::WORKFLOW_STATUS_COMPLETED.to_string(),
        input_fingerprint_sha256: None,
        input_fingerprint_json: Some(run_fingerprint_json),
        checkpoint_json: Some(json!({
            "schema_version": 1,
            "chapter_id": chapter.id,
            "chapter_number": chapter.chapter_number,
        })),
        artifacts_json: Some(serde_json::to_value(artifacts)?),
        generation_run_ids_json: None,
        error_message: None,
        completed: true,
    })
}

pub fn chapter_content_sync_checkpoint_artifacts(
    run: &WorkflowRunSummary,
    checkpoint: &WorkflowCheckpointSummary,
) -> Result<ChapterDerivedCleanupArtifacts, AppError> {
    if checkpoint.status != a4n_domain::workflow::WORKFLOW_STATUS_COMPLETED
        || checkpoint.input_fingerprint_json != run.input_fingerprint_json
    {
        return Err(AppError::InvalidState {
            current: format!("chapter_content_sync_run_id={}", run.id),
            operation: "章节资料同步 cleanup checkpoint 合同无效".to_string(),
        });
    }
    let contract = checkpoint
        .checkpoint_json
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("chapter_content_sync_run_id={}", run.id),
            operation: "章节资料同步 checkpoint_json 合同缺失".to_string(),
        })?;
    if contract.get("schema_version").and_then(Value::as_i64) != Some(1)
        || contract.get("chapter_id").and_then(Value::as_i64) != run.chapter_id
    {
        return Err(AppError::InvalidState {
            current: format!("chapter_content_sync_run_id={}", run.id),
            operation: "章节资料同步 checkpoint_json 身份无效".to_string(),
        });
    }
    checkpoint
        .artifacts_json
        .clone()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("chapter_content_sync_run_id={}", run.id),
            operation: "章节资料同步 checkpoint 缺少 cleanup artifacts".to_string(),
        })
        .and_then(|value| {
            serde_json::from_value(value).map_err(|error| AppError::InvalidState {
                current: format!("chapter_content_sync_run_id={}", run.id),
                operation: format!("章节资料同步 cleanup artifacts 合同无效: {error}"),
            })
        })
}

fn workflow_read_error(error: a4n_db::workflow::WorkflowReadError) -> AppError {
    match error {
        a4n_db::workflow::WorkflowReadError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::workflow::WorkflowReadError::NotFound(message) => AppError::NotFound(message),
        other => AppError::InvalidState {
            current: other.to_string(),
            operation: "读取章节资料同步 workflow".to_string(),
        },
    }
}

fn workflow_write_error(error: a4n_db::workflow::WorkflowWriteError) -> AppError {
    match error {
        a4n_db::workflow::WorkflowWriteError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::workflow::WorkflowWriteError::NotFound(message) => AppError::NotFound(message),
        a4n_db::workflow::WorkflowWriteError::StatusConflict { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        other => AppError::InvalidState {
            current: other.to_string(),
            operation: "写入章节资料同步 workflow".to_string(),
        },
    }
}
