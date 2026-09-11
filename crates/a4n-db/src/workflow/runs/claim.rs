use super::*;
use a4n_db_models::models::task::task_queue;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, DbBackend, Statement, TransactionTrait};

use super::mutations::{
    resume_workflow_run_model_in_transaction, upsert_workflow_run_in_transaction,
};
use super::queries::locked_workflow_run_by_id;
use super::validation::{
    should_resume_existing_run, validate_workflow_run_input, workflow_fingerprint_matches,
};

pub async fn claim_exclusive_workflow_run(
    db: &DatabaseConnection,
    input: ClaimExclusiveWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    let (project_id, chapter_id) = validate_claim_input(&input)?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    lock_video_generation_scope(db, &tx, project_id, chapter_id).await?;
    if let Some(running) = running_video_generation_workflow(&tx, project_id, chapter_id).await?
        && running_workflow_has_live_task(&tx, &running).await?
    {
        let current_step_key = running.current_step_key.as_deref().ok_or_else(|| {
            WorkflowWriteError::InvalidData(format!(
                "running 视频工作流 {} 缺少 current_step_key",
                running.id
            ))
        })?;
        let active_task_id = running.active_task_id.as_deref().ok_or_else(|| {
            WorkflowWriteError::InvalidData(format!(
                "running 视频工作流 {} 缺少 active_task_id",
                running.id
            ))
        })?;
        return Err(WorkflowWriteError::InvalidArgument(format!(
            "当前章节已有视频工作流正在运行：workflow_run_id={}，workflow_kind={}，current_step_key={}，active_task_id={}",
            running.id, running.workflow_kind, current_step_key, active_task_id
        )));
    }
    let run = match input.resume_from_task_id.as_deref() {
        Some(task_id) => resume_claimed_workflow_from_task(&tx, task_id, input.run).await?,
        None => upsert_workflow_run_in_transaction(&tx, input.run).await?,
    };
    persist_claimed_video_task(&tx, &run).await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(run)
}

async fn running_workflow_has_live_task(
    tx: &sea_orm::DatabaseTransaction,
    run: &workflow_run::Model,
) -> Result<bool, WorkflowWriteError> {
    let task = match run.active_task_id.as_deref() {
        Some(task_id) => task_queue::Entity::find_by_id(task_id.to_string())
            .one(tx)
            .await
            .map_err(WorkflowWriteError::Database)?,
        None => None,
    };
    if task
        .as_ref()
        .is_some_and(|task| matches!(task.status.as_str(), "pending" | "running"))
    {
        return Ok(true);
    }

    let owner_state = match task {
        Some(task) => format!("任务 {} 已处于终态 {}", task.id, task.status),
        None => run
            .active_task_id
            .as_deref()
            .map(|task_id| format!("任务 {task_id} 不存在"))
            .unwrap_or_else(|| "未绑定 active_task_id".to_string()),
    };
    let now = Utc::now().naive_utc();
    let mut active: workflow_run::ActiveModel = run.clone().into();
    active.status = Set(WORKFLOW_STATUS_INTERRUPTED.to_string());
    active.active_task_id = Set(None);
    active.can_resume = Set(true);
    active.resume_block_reason = Set(None);
    active.error_message = Set(Some(format!(
        "检测到视频工作流失去活动任务所有者（{owner_state}），已解除僵尸锁并保留断点"
    )));
    active.updated_at = Set(now);
    active.completed_at = Set(Some(now));
    active
        .update(tx)
        .await
        .map_err(WorkflowWriteError::Database)?;

    generation_run::Entity::update_many()
        .col_expr(
            generation_run::Column::Status,
            Expr::value(WORKFLOW_STATUS_INTERRUPTED.to_string()),
        )
        .col_expr(
            generation_run::Column::ErrorMessage,
            Expr::value(Some("所属视频工作流失去活动任务所有者".to_string())),
        )
        .col_expr(generation_run::Column::CompletedAt, Expr::value(Some(now)))
        .filter(generation_run::Column::WorkflowRunId.eq(run.id))
        .filter(generation_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .exec(tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    Ok(false)
}

async fn resume_claimed_workflow_from_task(
    tx: &sea_orm::DatabaseTransaction,
    task_id: &str,
    mut input: UpsertWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    let task = task_queue::Entity::find_by_id(task_id.to_string())
        .lock_exclusive()
        .one(tx)
        .await
        .map_err(WorkflowWriteError::Database)?
        .ok_or_else(|| WorkflowWriteError::NotFound(format!("重试来源任务不存在: {task_id}")))?;
    if !is_terminal_task_status(&task.status) {
        return Err(WorkflowWriteError::InvalidArgument(format!(
            "重试来源任务 {task_id} 尚未结束，当前状态为 {}",
            task.status
        )));
    }
    let workflow_run_id = task.workflow_run_id.ok_or_else(|| {
        WorkflowWriteError::InvalidData(format!("重试来源任务 {task_id} 未绑定 workflow_run"))
    })?;
    let existing = locked_workflow_run_by_id(tx, workflow_run_id).await?;
    if existing.business_key != input.business_key
        || existing.workflow_kind != input.workflow_kind
        || existing.project_id != input.project_id
        || existing.root_project_id != input.root_project_id
        || existing.chapter_id != input.chapter_id
        || existing.mode != input.mode
        || !workflow_fingerprint_matches(
            existing.input_fingerprint_json.as_ref(),
            input.input_fingerprint_json.as_ref(),
        )
    {
        return Err(WorkflowWriteError::InvalidArgument(format!(
            "重试来源任务 {task_id} 与当前视频生成请求不一致"
        )));
    }
    if !should_resume_existing_run(&existing, &input.status) {
        return Err(WorkflowWriteError::StatusConflict {
            current: existing.status.clone(),
            operation: format!("通过任务 {task_id} 恢复工作流"),
        });
    }
    input.current_step_key = existing.current_step_key.clone();
    resume_workflow_run_model_in_transaction(tx, existing, input).await
}

async fn persist_claimed_video_task(
    tx: &sea_orm::DatabaseTransaction,
    run: &workflow_run::Model,
) -> Result<(), WorkflowWriteError> {
    let task_id = run.active_task_id.as_deref().ok_or_else(|| {
        WorkflowWriteError::InvalidData(format!("独占视频工作流 {} 缺少 active_task_id", run.id))
    })?;
    let now = Utc::now().naive_utc();
    if let Some(existing) = task_queue::Entity::find_by_id(task_id.to_string())
        .one(tx)
        .await
        .map_err(WorkflowWriteError::Database)?
    {
        if let Some(previous_run_id) = existing.workflow_run_id.filter(|id| *id != run.id) {
            let previous = workflow_run::Entity::find_by_id(previous_run_id)
                .one(tx)
                .await
                .map_err(WorkflowWriteError::Database)?
                .ok_or_else(|| {
                    WorkflowWriteError::InvalidData(format!(
                        "视频任务 {task_id} 绑定的工作流 {previous_run_id} 不存在"
                    ))
                })?;
            if previous.status != WORKFLOW_STATUS_COMPLETED {
                return Err(WorkflowWriteError::InvalidArgument(format!(
                    "视频任务 {task_id} 已绑定未完成工作流 {previous_run_id}"
                )));
            }
        }
        if is_terminal_task_status(&existing.status) {
            return Err(WorkflowWriteError::InvalidArgument(format!(
                "视频任务 {task_id} 已结束，不能重新绑定运行中的工作流"
            )));
        }
        let mut active: task_queue::ActiveModel = existing.into();
        active.status = Set(WORKFLOW_STATUS_RUNNING.to_string());
        active.project_id = Set(run.project_id);
        active.root_project_id = Set(run.root_project_id.or(run.project_id));
        active.source_snapshot_id = Set(run.source_snapshot_id);
        active.task_scope = Set(Some("video_prompt".to_string()));
        active.workflow_run_id = Set(Some(run.id));
        active.error_message = Set(None);
        active.started_at = Set(Some(now));
        active.completed_at = Set(None);
        active
            .update(tx)
            .await
            .map_err(WorkflowWriteError::Database)?;
        return Ok(());
    }

    task_queue::ActiveModel {
        id: Set(task_id.to_string()),
        task_type: Set("llm_call".to_string()),
        priority: Set("medium".to_string()),
        status: Set(WORKFLOW_STATUS_RUNNING.to_string()),
        project_id: Set(run.project_id),
        root_project_id: Set(run.root_project_id.or(run.project_id)),
        source_snapshot_id: Set(run.source_snapshot_id),
        task_scope: Set(Some("video_prompt".to_string())),
        workflow_run_id: Set(Some(run.id)),
        payload: Set(None),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(None),
        created_at: Set(now),
        started_at: Set(Some(now)),
        completed_at: Set(None),
    }
    .insert(tx)
    .await
    .map_err(WorkflowWriteError::Database)?;
    Ok(())
}

fn validate_claim_input(
    input: &ClaimExclusiveWorkflowRunInput,
) -> Result<(i64, i64), WorkflowWriteError> {
    validate_workflow_run_input(&input.run)?;
    if input.run.status != WORKFLOW_STATUS_RUNNING {
        return Err(WorkflowWriteError::InvalidArgument(
            "独占 workflow claim 只能创建 running 状态".to_string(),
        ));
    }
    if input
        .run
        .active_task_id
        .as_deref()
        .and_then(non_empty_text)
        .is_none()
    {
        return Err(WorkflowWriteError::InvalidArgument(
            "独占 workflow claim 必须绑定 active_task_id".to_string(),
        ));
    }
    if input.resume_from_task_id.is_some()
        && input
            .resume_from_task_id
            .as_deref()
            .and_then(non_empty_text)
            .is_none()
    {
        return Err(WorkflowWriteError::InvalidArgument(
            "resume_from_task_id 不能为空".to_string(),
        ));
    }
    let project_id = input.run.project_id.ok_or_else(|| {
        WorkflowWriteError::InvalidArgument("独占 workflow claim 缺少 project_id".to_string())
    })?;
    let chapter_id = input.run.chapter_id.ok_or_else(|| {
        WorkflowWriteError::InvalidArgument("独占 workflow claim 缺少 chapter_id".to_string())
    })?;
    Ok((project_id, chapter_id))
}

async fn lock_video_generation_scope(
    db: &DatabaseConnection,
    tx: &sea_orm::DatabaseTransaction,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), WorkflowWriteError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(WorkflowWriteError::InvalidData(
            "视频工作流独占 claim 仅支持 PostgreSQL".to_string(),
        ));
    }
    let exclusion_scope = format!("video_prompt:{project_id}:{chapter_id}");
    tx.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        [exclusion_scope.into()],
    ))
    .await
    .map_err(WorkflowWriteError::Database)?;
    Ok(())
}

async fn running_video_generation_workflow(
    tx: &sea_orm::DatabaseTransaction,
    project_id: i64,
    chapter_id: i64,
) -> Result<Option<workflow_run::Model>, WorkflowWriteError> {
    workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::ChapterId.eq(chapter_id))
        .filter(workflow_run::Column::WorkflowKind.is_in([
            "video_segment_plan",
            "video_frame_prompts",
            "video_prompts",
        ]))
        .filter(workflow_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .lock_exclusive()
        .one(tx)
        .await
        .map_err(WorkflowWriteError::Database)
}

fn is_terminal_task_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled" | "interrupted")
}
