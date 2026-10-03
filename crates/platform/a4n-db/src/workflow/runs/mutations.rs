use super::*;

use super::super::llm_checkpoint::inherit_completed_llm_call_checkpoints;
use super::projections::workflow_run_summary;
use super::queries::{locked_latest_workflow_run_by_business_key_model, locked_workflow_run_by_id};
use super::validation::{
    completed_at_for_status_value, ensure_workflow_status_transition, ensure_workflow_write_open,
    requires_explicit_interrupted_resume, should_resume_existing_run, should_update_existing_run,
    validate_workflow_run_input, validate_workflow_status_fields, workflow_fingerprint_matches,
};
use a4n_db_models::models::{
    chapter::chapter, generation::generation_task_checkpoint, task::task_queue,
};
use sea_orm::sea_query::LockType;
use sea_orm::{Condition, DbBackend, DbErr, QuerySelect};

const CHAPTER_ANALYSIS_WORKFLOW_KIND: &str = "chapter_analysis";
const CHAPTER_ANALYSIS_DERIVED_CHECKPOINTS: &[&str] = &[
    a4n_domain::fact_feedback::CHAPTER_FACT_FEEDBACK_STEP,
    a4n_domain::fact_feedback::CHAPTER_FACT_INDEX_STEP,
    "chapter_analysis.derived_sync",
    "chapter_analysis.outline_sync",
    "chapter_analysis.sync_outputs",
    "chapter_analysis.object_discovery",
    "chapter_analysis.character_discovery",
    "chapter_analysis.scene_sync",
    "chapter_analysis.character_sync",
    "chapter_analysis.refresh_status",
    "chapter_analysis.replacement_cleanup",
    "chapter_analysis.replacement_commit",
];

/// 正文在某章发生变化后，使该章及后续章节的派生分析重新进入可恢复状态。
///
/// LLM 阶段 checkpoint 保留，供恢复时复用；资料同步 checkpoint 和每章的
/// 角色/场景投影 checkpoint 删除，避免把已回退的全局资料误报为 completed。
pub async fn invalidate_chapter_analysis_after_source_change<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
    preserve_workflow_run_id: Option<i64>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if project_id <= 0 || chapter_number <= 0 {
        return Err(DbErr::Custom(
            "章节分析失效范围必须使用正 project_id 和 chapter_number".to_string(),
        ));
    }
    let chapter_ids = chapter::Entity::find_current()
        .select_only()
        .column(chapter::Column::Id)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.gte(chapter_number))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if chapter_ids.is_empty() {
        return Ok(());
    }

    let runs = workflow_run::Entity::find()
        .filter(workflow_run::Column::WorkflowKind.eq(CHAPTER_ANALYSIS_WORKFLOW_KIND))
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::ChapterId.is_in(chapter_ids))
        .filter(workflow_run::Column::Status.ne(WORKFLOW_STATUS_SUPERSEDED))
        .lock(LockType::Update)
        .all(db)
        .await?;
    for run in runs {
        let mut step_condition = Condition::any();
        for step_key in CHAPTER_ANALYSIS_DERIVED_CHECKPOINTS {
            step_condition = step_condition.add(workflow_checkpoint::Column::StepKey.eq(*step_key));
        }
        step_condition = step_condition
            .add(
                workflow_checkpoint::Column::StepKey
                    .like("chapter_analysis.character_sync.profile.%"),
            )
            .add(workflow_checkpoint::Column::StepKey.like("scene_profile_sync.chapter.%"));
        workflow_checkpoint::Entity::delete_many()
            .filter(workflow_checkpoint::Column::WorkflowRunId.eq(run.id))
            .filter(step_condition)
            .exec(db)
            .await?;

        if preserve_workflow_run_id == Some(run.id) {
            continue;
        }
        let now = Utc::now().naive_utc();
        let mut active: workflow_run::ActiveModel = run.into();
        active.status = Set(WORKFLOW_STATUS_INTERRUPTED.to_string());
        active.mode = Set(Some(
            a4n_domain::workflow::WORKFLOW_MODE_DERIVED_REPLAY.to_string(),
        ));
        active.current_step_key = Set(Some("chapter_analysis.derived_sync".to_string()));
        active.active_task_id = Set(None);
        active.can_resume = Set(true);
        active.resume_block_reason = Set(None);
        active.error_message = Set(Some("正文变化后等待重新同步章节派生资料".to_string()));
        active.updated_at = Set(now);
        active.completed_at = Set(Some(now));
        active.update(db).await?;
    }
    Ok(())
}

pub async fn upsert_workflow_run(
    db: &DatabaseConnection,
    input: UpsertWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = upsert_workflow_run_in_transaction(&tx, input).await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(run)
}

pub async fn upsert_workflow_run_summary(
    db: &DatabaseConnection,
    input: UpsertWorkflowRunInput,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    upsert_workflow_run(db, input)
        .await
        .map_err(WorkflowWriteError::from)
        .and_then(|run| workflow_run_summary(run).map_err(WorkflowWriteError::from))
}

pub async fn resume_or_upsert_workflow_run(
    db: &DatabaseConnection,
    input: UpsertWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_workflow_run_input(&input)?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    // A missing workflow row cannot be locked. Serialize project starts before checking the business key.
    if let Some(project_id) = input.project_id {
        crate::project::project_write_identity_for_update(&tx, project_id)
            .await
            .map_err(WorkflowWriteError::Database)?
            .ok_or_else(|| WorkflowWriteError::NotFound(format!("项目不存在: {project_id}")))?;
    }
    let existing =
        locked_latest_workflow_run_by_business_key_model(&tx, &input.business_key).await?;
    if let Some(existing) = existing.as_ref()
        && existing.status == WORKFLOW_STATUS_RUNNING
    {
        return Err(WorkflowWriteError::StatusConflict {
            current: existing.status.clone(),
            operation: "重复启动同一工作流".to_string(),
        });
    }
    if let Some(existing) = existing.filter(|existing| {
        existing.workflow_kind == input.workflow_kind
            && workflow_fingerprint_matches(
                existing.input_fingerprint_json.as_ref(),
                input.input_fingerprint_json.as_ref(),
            )
            && should_resume_existing_run(existing, &input.status)
    }) {
        let run = resume_workflow_run_model_in_transaction(&tx, existing, input).await?;
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(run);
    }
    let run = upsert_workflow_run_in_transaction(&tx, input).await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(run)
}

pub async fn resume_or_upsert_workflow_run_summary(
    db: &DatabaseConnection,
    input: UpsertWorkflowRunInput,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    resume_or_upsert_workflow_run(db, input)
        .await
        .map_err(WorkflowWriteError::from)
        .and_then(|run| workflow_run_summary(run).map_err(WorkflowWriteError::from))
}

pub async fn continue_completed_workflow_run(
    db: &DatabaseConnection,
    source_run_id: i64,
    input: UpsertWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_workflow_run_input(&input)?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    if let Some(project_id) = input.project_id {
        crate::project::project_write_identity_for_update(&tx, project_id)
            .await
            .map_err(WorkflowWriteError::Database)?
            .ok_or_else(|| WorkflowWriteError::NotFound(format!("项目不存在: {project_id}")))?;
    }
    let source = locked_latest_workflow_run_by_business_key_model(&tx, &input.business_key)
        .await?
        .ok_or_else(|| WorkflowWriteError::NotFound("待继续的工作流不存在".into()))?;
    if source.id != source_run_id
        || source.status != WORKFLOW_STATUS_COMPLETED
        || input.status != WORKFLOW_STATUS_RUNNING
        || source.project_id != input.project_id
        || source.chapter_id != input.chapter_id
        || source.workflow_kind != input.workflow_kind
        || !workflow_fingerprint_matches(
            source.input_fingerprint_json.as_ref(),
            input.input_fingerprint_json.as_ref(),
        )
    {
        return Err(WorkflowWriteError::InvalidData(
            "继续阶段的工作流来源已变化".into(),
        ));
    }
    let run = upsert_workflow_run_in_transaction(&tx, input).await?;
    inherit_completed_llm_call_checkpoints(&tx, source_run_id, run.id).await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(run)
}

pub async fn continue_completed_workflow_run_summary(
    db: &DatabaseConnection,
    source_run_id: i64,
    input: UpsertWorkflowRunInput,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    continue_completed_workflow_run(db, source_run_id, input)
        .await
        .map_err(WorkflowWriteError::from)
        .and_then(|run| workflow_run_summary(run).map_err(WorkflowWriteError::from))
}

pub(super) async fn resume_workflow_run_model_in_transaction<C: ConnectionTrait>(
    tx: &C,
    existing: workflow_run::Model,
    input: UpsertWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    ensure_workflow_status_transition(&existing.status, &input.status, true)?;
    let now = Utc::now().naive_utc();
    let mut active: workflow_run::ActiveModel = existing.into();
    active.status = Set(input.status);
    active.current_step_key = Set(normalize_optional_text(input.current_step_key));
    active.active_task_id = Set(normalize_optional_text(input.active_task_id));
    active.can_resume = Set(input.can_resume);
    active.resume_block_reason = Set(None);
    active.error_message = Set(None);
    active.updated_at = Set(now);
    active.completed_at = Set(None);
    active
        .update(tx)
        .await
        .map_err(WorkflowWriteError::Database)
}

pub async fn upsert_workflow_run_in_transaction<C: ConnectionTrait>(
    tx: &C,
    input: UpsertWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_workflow_run_input(&input)?;
    let now = Utc::now().naive_utc();
    let existing =
        locked_latest_workflow_run_by_business_key_model(tx, &input.business_key).await?;
    if let Some(existing) = existing.as_ref()
        && workflow_fingerprint_matches(
            existing.input_fingerprint_json.as_ref(),
            input.input_fingerprint_json.as_ref(),
        )
        && requires_explicit_interrupted_resume(existing, &input.status)
    {
        return Err(WorkflowWriteError::StatusConflict {
            current: existing.status.clone(),
            operation: format!("将工作流状态更新为 {}", input.status),
        });
    }
    if let Some(existing) = existing.as_ref()
        && workflow_fingerprint_matches(
            existing.input_fingerprint_json.as_ref(),
            input.input_fingerprint_json.as_ref(),
        )
        && is_terminal_workflow_status(&existing.status)
        && input.status != WORKFLOW_STATUS_RUNNING
        && existing.status != input.status
    {
        return Err(WorkflowWriteError::StatusConflict {
            current: existing.status.clone(),
            operation: format!("将工作流状态更新为 {}", input.status),
        });
    }
    if let Some(existing) = existing.as_ref()
        && workflow_fingerprint_matches(
            existing.input_fingerprint_json.as_ref(),
            input.input_fingerprint_json.as_ref(),
        )
        && should_update_existing_run(existing, &input.status)
    {
        let mut active: workflow_run::ActiveModel = existing.clone().into();
        active.workflow_kind = Set(input.workflow_kind);
        active.project_id = Set(input.project_id);
        active.root_project_id = Set(input.root_project_id);
        active.source_snapshot_id = Set(input.source_snapshot_id);
        active.scope_table = Set(normalize_optional_text(input.scope_table));
        active.scope_id = Set(input.scope_id);
        active.chapter_id = Set(input.chapter_id);
        active.mode = Set(normalize_optional_text(input.mode));
        active.status = Set(input.status.clone());
        active.current_step_key = Set(normalize_optional_text(input.current_step_key));
        active.active_task_id = Set(normalize_optional_text(input.active_task_id));
        active.can_resume = Set(input.can_resume);
        active.resume_block_reason = Set(normalize_optional_text(input.resume_block_reason));
        active.error_message = Set(normalize_optional_text(input.error_message));
        active.updated_at = Set(now);
        active.completed_at = Set(completed_at_for_status_value(&input.status, now));
        return active
            .update(tx)
            .await
            .map_err(WorkflowWriteError::Database);
    }

    let checkpoint_source = existing
        .as_ref()
        .filter(|previous| {
            input.status == WORKFLOW_STATUS_RUNNING
                && input.can_resume
                && matches!(
                    previous.status.as_str(),
                    WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_CANCELLED
                )
                && previous.workflow_kind == input.workflow_kind
                && workflow_fingerprint_matches(
                    previous.input_fingerprint_json.as_ref(),
                    input.input_fingerprint_json.as_ref(),
                )
        })
        .map(|previous| previous.id);
    if let Some(existing) = existing
        && !is_terminal_workflow_status(&existing.status)
    {
        let mut active: workflow_run::ActiveModel = existing.into();
        active.status = Set(WORKFLOW_STATUS_SUPERSEDED.to_string());
        active.active_task_id = Set(None);
        active.can_resume = Set(false);
        active.resume_block_reason = Set(Some("输入指纹变化，旧工作流已被新运行取代".to_string()));
        active.updated_at = Set(now);
        active.completed_at = Set(Some(now));
        active
            .update(tx)
            .await
            .map_err(WorkflowWriteError::Database)?;
    }

    let run = workflow_run::ActiveModel {
        id: NotSet,
        business_key: Set(input.business_key),
        workflow_kind: Set(input.workflow_kind),
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        scope_table: Set(normalize_optional_text(input.scope_table)),
        scope_id: Set(input.scope_id),
        chapter_id: Set(input.chapter_id),
        mode: Set(normalize_optional_text(input.mode)),
        input_fingerprint_json: Set(input.input_fingerprint_json),
        status: Set(input.status.clone()),
        current_step_key: Set(normalize_optional_text(input.current_step_key)),
        progress_json: Set(None),
        active_task_id: Set(normalize_optional_text(input.active_task_id)),
        can_resume: Set(input.can_resume),
        resume_block_reason: Set(normalize_optional_text(input.resume_block_reason)),
        error_message: Set(normalize_optional_text(input.error_message)),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(completed_at_for_status_value(&input.status, now)),
    }
    .insert(tx)
    .await
    .map_err(WorkflowWriteError::Database)?;
    if let Some(source_id) = checkpoint_source {
        // 新尝试保留独立运行记录；恢复资源的粒度始终是已完成的模型调用。
        inherit_completed_llm_call_checkpoints(tx, source_id, run.id).await?;
    }
    Ok(run)
}

pub async fn upsert_workflow_run_summary_in_transaction<C: ConnectionTrait>(
    tx: &C,
    input: UpsertWorkflowRunInput,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    let run = upsert_workflow_run_in_transaction(tx, input).await?;
    workflow_run_summary(run).map_err(WorkflowWriteError::from)
}

pub async fn mark_workflow_status(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    status: &str,
    can_resume: bool,
    error_message: Option<&str>,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    validate_workflow_status(status)?;
    validate_workflow_status_fields(status, can_resume, error_message, error_message)?;
    let error_message = error_message.and_then(non_empty_text);
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    ensure_workflow_status_transition(&run.status, status, false)?;
    if is_terminal_workflow_status(&run.status) && run.status == status {
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(run);
    }
    let now = Utc::now().naive_utc();
    let mut progress_json = run.progress_json.clone();
    if status == WORKFLOW_STATUS_INTERRUPTED
        && let Some(active_task_id) = run
            .active_task_id
            .as_deref()
            .map(str::trim)
            .filter(|task_id| !task_id.is_empty())
    {
        let mut progress = progress_json.unwrap_or_else(|| serde_json::json!({}));
        let fields = progress.as_object_mut().ok_or_else(|| {
            WorkflowWriteError::InvalidData("progress_json 必须是 JSON object".to_string())
        })?;
        fields.insert(
            "cancellation_active_task_id".to_string(),
            serde_json::json!(active_task_id),
        );
        progress_json = Some(progress);
    }
    let mut active: workflow_run::ActiveModel = run.into();
    active.status = Set(status.to_string());
    active.can_resume = Set(can_resume);
    active.resume_block_reason = Set((!can_resume).then(|| error_message.clone()).flatten());
    active.error_message = Set(error_message);
    active.active_task_id = Set(None);
    active.progress_json = Set(progress_json);
    active.updated_at = Set(now);
    active.completed_at = Set(completed_at_for_status_value(status, now));
    let updated = active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(updated)
}

pub async fn mark_workflow_status_summary(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    status: &str,
    can_resume: bool,
    error_message: Option<&str>,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    mark_workflow_status(db, workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(WorkflowWriteError::from)
        .and_then(|run| workflow_run_summary(run).map_err(WorkflowWriteError::from))
}

pub async fn resume_workflow_run(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    resume_workflow_run_inner(db, workflow_run_id, None).await
}

pub async fn resume_workflow_run_summary(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    resume_workflow_run(db, workflow_run_id)
        .await
        .map_err(WorkflowWriteError::from)
        .and_then(|run| workflow_run_summary(run).map_err(WorkflowWriteError::from))
}

/// 将可恢复 workflow 绑定到已经预留的新执行任务。
///
/// generation reservation 与 worker 启动是异步的；恢复入口必须先把这两个
/// durable identity 绑定，否则接口返回时仍会读到旧终态，worker 随后又可能
/// 把同一 workflow 当作重复启动。
pub async fn resume_workflow_run_with_active_task(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    active_task_id: &str,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    let active_task_id = required_text(active_task_id.to_string(), "active_task_id")?;
    resume_workflow_run_inner(db, workflow_run_id, Some(&active_task_id)).await
}

pub async fn resume_workflow_run_with_active_task_summary(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    active_task_id: &str,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    resume_workflow_run_with_active_task(db, workflow_run_id, active_task_id)
        .await
        .map_err(WorkflowWriteError::from)
        .and_then(|run| workflow_run_summary(run).map_err(WorkflowWriteError::from))
}

async fn resume_workflow_run_inner(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    active_task_id: Option<&str>,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let active_task = if let Some(active_task_id) = active_task_id {
        let mut query = task_queue::Entity::find_by_id(active_task_id.to_string());
        if tx.get_database_backend() != DbBackend::Sqlite {
            query = query.lock_exclusive();
        }
        Some(
            query
                .one(&tx)
                .await
                .map_err(WorkflowWriteError::Database)?
                .ok_or_else(|| {
                    WorkflowWriteError::NotFound(format!(
                        "恢复工作流的活动任务不存在: {active_task_id}"
                    ))
                })?,
        )
    } else {
        None
    };
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    if let (Some(active_task_id), Some(active_task)) = (active_task_id, active_task.as_ref()) {
        validate_and_bind_active_task(&tx, workflow_run_id, active_task_id, active_task).await?;
    }
    if run.status == WORKFLOW_STATUS_RUNNING {
        if active_task_id.is_some() && run.active_task_id.as_deref() == active_task_id {
            tx.commit().await.map_err(WorkflowWriteError::Database)?;
            return Ok(run);
        }
        return Err(WorkflowWriteError::StatusConflict {
            current: run.status,
            operation: "重复恢复工作流".to_string(),
        });
    }
    ensure_workflow_status_transition(&run.status, WORKFLOW_STATUS_RUNNING, true)?;
    let mut active: workflow_run::ActiveModel = run.into();
    active.status = Set(WORKFLOW_STATUS_RUNNING.to_string());
    active.can_resume = Set(true);
    active.resume_block_reason = Set(None);
    active.error_message = Set(None);
    active.active_task_id = Set(active_task_id.map(ToString::to_string));
    active.updated_at = Set(Utc::now().naive_utc());
    active.completed_at = Set(None);
    let updated = active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(updated)
}

async fn validate_and_bind_active_task(
    tx: &sea_orm::DatabaseTransaction,
    workflow_run_id: i64,
    active_task_id: &str,
    task: &task_queue::Model,
) -> Result<(), WorkflowWriteError> {
    if !is_live_active_task_status(&task.status) {
        return Err(WorkflowWriteError::StatusConflict {
            current: format!("task_queue:{}", task.status),
            operation: "恢复工作流并绑定活动任务".to_string(),
        });
    }
    if task
        .workflow_run_id
        .is_some_and(|current| current != workflow_run_id)
    {
        return Err(WorkflowWriteError::StatusConflict {
            current: format!("task_queue.workflow_run_id={:?}", task.workflow_run_id),
            operation: format!("将活动任务 {active_task_id} 绑定到 workflow {workflow_run_id}"),
        });
    }
    if task.workflow_run_id != Some(workflow_run_id) {
        let mut active: task_queue::ActiveModel = task.clone().into();
        active.workflow_run_id = Set(Some(workflow_run_id));
        active
            .update(tx)
            .await
            .map_err(WorkflowWriteError::Database)?;
    }
    if generation_task_checkpoint::Entity::find_by_id(active_task_id.to_string())
        .one(tx)
        .await
        .map_err(WorkflowWriteError::Database)?
        .is_some()
    {
        crate::generation::bind_generation_task_to_workflow(tx, active_task_id, workflow_run_id)
            .await
            .map_err(|error| match error {
                crate::generation::GenerationTaskPersistenceError::Database(error) => {
                    WorkflowWriteError::Database(error)
                }
                error => WorkflowWriteError::InvalidData(format!(
                    "绑定 generation task {active_task_id} 到 workflow {workflow_run_id} 失败: {error}"
                )),
            })?;
    } else {
        crate::queue::bind_locked_task_queue_row_to_workflow(tx, task.clone(), workflow_run_id)
            .await
            .map_err(|error| match error {
                crate::queue::TaskWorkflowBindingError::Conflict { current } => {
                    WorkflowWriteError::StatusConflict {
                        current: format!("task_queue.workflow_run_id={current}"),
                        operation: format!(
                            "将活动任务 {active_task_id} 绑定到 workflow {workflow_run_id}"
                        ),
                    }
                }
                crate::queue::TaskWorkflowBindingError::Database(error) => {
                    WorkflowWriteError::Database(error)
                }
            })?;
    }
    Ok(())
}

fn is_live_active_task_status(status: &str) -> bool {
    matches!(status, "pending" | "running")
}

#[cfg(test)]
mod tests {
    use super::is_live_active_task_status;

    #[test]
    fn only_pending_and_running_tasks_can_be_bound_as_active() {
        assert!(is_live_active_task_status("pending"));
        assert!(is_live_active_task_status("running"));
        assert!(!is_live_active_task_status("completed"));
        assert!(!is_live_active_task_status("cancelled"));
    }
}

pub async fn mark_workflow_current_step(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    current_step_key: &str,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let current_step_key = required_text(current_step_key.to_string(), "current_step_key")?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    ensure_workflow_write_open(&run.status, "更新工作流当前步骤")?;
    let now = Utc::now().naive_utc();
    let mut active: workflow_run::ActiveModel = run.into();
    active.current_step_key = Set(Some(current_step_key));
    active.updated_at = Set(now);
    let updated = active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(updated)
}

pub async fn mark_workflow_current_step_summary(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    current_step_key: &str,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    mark_workflow_current_step(db, workflow_run_id, current_step_key)
        .await
        .map_err(WorkflowWriteError::from)
        .and_then(|run| workflow_run_summary(run).map_err(WorkflowWriteError::from))
}
