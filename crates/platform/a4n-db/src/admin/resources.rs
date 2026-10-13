use a4n_db_models::models::project::novel_project;
use a4n_db_models::models::task::task_queue;
use a4n_domain::project::{ProjectKind, ProjectStatus};
use sea_orm::{ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect};

use super::{AdminDbError, AdminProjectSummary, PendingTaskIndexSummary, TaskQueueSummary};

#[derive(Debug, FromQueryResult)]
struct AdminProjectSummaryRow {
    id: i64,
    user_id: i64,
    title: String,
    project_type: String,
    project_kind: String,
    status: String,
    target_chapters: i32,
    current_chapter: i32,
    total_chapters: Option<i32>,
}

#[derive(Debug, FromQueryResult)]
struct PendingTaskIndexRow {
    id: String,
    task_type: String,
    priority: String,
    status: String,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
    task_scope: Option<String>,
    dag_id: Option<String>,
    created_at: chrono::NaiveDateTime,
}

pub async fn pending_tasks(
    db: &impl crate::DbExecutor,
) -> Result<Vec<TaskQueueSummary>, AdminDbError> {
    let mut tasks = task_queue::Entity::find()
        .filter(task_queue::Column::Status.eq("pending"))
        .all(db)
        .await?;
    tasks.sort_by(|a, b| {
        priority_rank(&a.priority)
            .cmp(&priority_rank(&b.priority))
            .then(a.created_at.cmp(&b.created_at))
            .then(a.id.cmp(&b.id))
    });
    Ok(tasks.iter().map(task_summary).collect())
}

pub async fn pending_task_index(
    db: &impl crate::DbExecutor,
) -> Result<Vec<PendingTaskIndexSummary>, AdminDbError> {
    let mut tasks = task_queue::Entity::find()
        .select_only()
        .column(task_queue::Column::Id)
        .column(task_queue::Column::TaskType)
        .column(task_queue::Column::Priority)
        .column(task_queue::Column::Status)
        .column(task_queue::Column::ProjectId)
        .column(task_queue::Column::RootProjectId)
        .column(task_queue::Column::SourceSnapshotId)
        .column(task_queue::Column::TaskScope)
        .column(task_queue::Column::DagId)
        .column(task_queue::Column::CreatedAt)
        .filter(task_queue::Column::Status.eq("pending"))
        .into_model::<PendingTaskIndexRow>()
        .all(db)
        .await?;
    tasks.sort_by(|a, b| {
        priority_rank(&a.priority)
            .cmp(&priority_rank(&b.priority))
            .then(a.created_at.cmp(&b.created_at))
            .then(a.id.cmp(&b.id))
    });
    Ok(tasks
        .into_iter()
        .map(|task| PendingTaskIndexSummary {
            id: task.id,
            task_type: task.task_type,
            priority: task.priority,
            status: task.status,
            project_id: task.project_id,
            root_project_id: task.root_project_id,
            source_snapshot_id: task.source_snapshot_id,
            task_scope: task.task_scope,
            dag_id: task.dag_id,
            created_at: task.created_at.and_utc().to_rfc3339(),
        })
        .collect())
}

pub async fn all_project_summaries(
    db: &impl crate::DbExecutor,
) -> Result<Vec<AdminProjectSummary>, AdminDbError> {
    let projects = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::UserId)
        .column(novel_project::Column::Title)
        .column(novel_project::Column::ProjectType)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::Status)
        .column(novel_project::Column::TargetChapters)
        .column(novel_project::Column::CurrentChapter)
        .column(novel_project::Column::TotalChapters)
        .order_by_desc(novel_project::Column::CreatedAt)
        .into_model::<AdminProjectSummaryRow>()
        .all(db)
        .await?;
    projects.iter().map(project_summary).collect()
}

pub(super) fn priority_rank(priority: &str) -> u8 {
    match priority {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        _ => 3,
    }
}

fn task_summary(task: &task_queue::Model) -> TaskQueueSummary {
    TaskQueueSummary {
        id: task.id.clone(),
        task_type: task.task_type.clone(),
        priority: task.priority.clone(),
        status: task.status.clone(),
        project_id: task.project_id,
        root_project_id: task.root_project_id,
        source_snapshot_id: task.source_snapshot_id,
        task_scope: task.task_scope.clone(),
        payload: task.payload.clone(),
        progress: task.progress.clone(),
        error_message: task.error_message.clone(),
        dag_id: task.dag_id.clone(),
        created_at: Some(task.created_at.and_utc().to_rfc3339()),
        started_at: task.started_at.map(|value| value.and_utc().to_rfc3339()),
        completed_at: task.completed_at.map(|value| value.and_utc().to_rfc3339()),
    }
}

fn project_summary(project: &AdminProjectSummaryRow) -> Result<AdminProjectSummary, AdminDbError> {
    let project_kind = ProjectKind::parse(&project.project_kind).ok_or_else(|| {
        AdminDbError::InvalidArgument(format!(
            "项目 #{} 的 project_kind 无效: {}",
            project.id, project.project_kind
        ))
    })?;
    let status = ProjectStatus::parse(&project.status).ok_or_else(|| {
        AdminDbError::InvalidArgument(format!(
            "项目 #{} 的 status 无效: {}",
            project.id, project.status
        ))
    })?;
    Ok(AdminProjectSummary {
        id: project.id,
        user_id: project.user_id,
        title: project.title.clone(),
        project_type: project.project_type.clone(),
        project_kind,
        status,
        target_chapters: project.target_chapters,
        current_chapter: project.current_chapter,
        total_chapters: project.total_chapters,
    })
}
