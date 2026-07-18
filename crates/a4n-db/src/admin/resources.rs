use a4n_core::models::project::novel_project;
use a4n_core::models::task::task_queue;
use a4n_core::models::user::{invite_code, user};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use super::{AdminDbError, AdminUserSummary, InviteCodeSummary, ProjectSummary, TaskQueueSummary};

pub async fn all_user_summaries(
    db: &DatabaseConnection,
) -> Result<Vec<AdminUserSummary>, AdminDbError> {
    let users = user::Entity::find()
        .order_by_asc(user::Column::Id)
        .all(db)
        .await?;
    Ok(users.iter().map(admin_user_summary).collect())
}

pub async fn invite_code_summaries(
    db: &DatabaseConnection,
) -> Result<Vec<InviteCodeSummary>, AdminDbError> {
    let codes = invite_code::Entity::find()
        .order_by_desc(invite_code::Column::CreatedAt)
        .order_by_desc(invite_code::Column::Id)
        .all(db)
        .await?;
    Ok(codes.iter().map(invite_code_summary).collect())
}

pub async fn pending_tasks(db: &DatabaseConnection) -> Result<Vec<TaskQueueSummary>, AdminDbError> {
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

pub async fn all_project_summaries(
    db: &DatabaseConnection,
) -> Result<Vec<ProjectSummary>, AdminDbError> {
    let projects = novel_project::Entity::find()
        .order_by_desc(novel_project::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(projects.iter().map(project_summary).collect())
}

pub(super) fn priority_rank(priority: &str) -> u8 {
    match priority {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        _ => 3,
    }
}

pub(super) fn admin_user_summary(user: &user::Model) -> AdminUserSummary {
    AdminUserSummary {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        oauth_provider: user.oauth_provider.clone(),
        oauth_subject: user.oauth_subject.clone(),
        avatar_url: user.avatar_url.clone(),
        is_active: user.is_active,
        display_name: Some(user.username.clone()),
        created_at: Some(user.created_at.to_string()),
        updated_at: Some(user.updated_at.to_string()),
        last_login_at: user.last_login_at.map(|value| value.to_string()),
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
        created_at: Some(task.created_at.to_string()),
        started_at: task.started_at.map(|value| value.to_string()),
        completed_at: task.completed_at.map(|value| value.to_string()),
    }
}

pub(super) fn invite_code_summary(code: &invite_code::Model) -> InviteCodeSummary {
    InviteCodeSummary {
        id: code.id,
        code: code.code.clone(),
        created_by_user_id: code.created_by_user_id,
        used_by_user_id: code.used_by_user_id,
        created_at: Some(code.created_at.to_string()),
        used_at: code.used_at.map(|value| value.to_string()),
    }
}

fn project_summary(project: &novel_project::Model) -> ProjectSummary {
    ProjectSummary {
        id: project.id,
        user_id: project.user_id,
        title: project.title.clone(),
        description: project.description.clone(),
        project_type: project.project_type.clone(),
        status: project.status.clone(),
        target_chapters: project.target_chapters,
        current_chapter: project.current_chapter,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        avatar_animal: project.avatar_animal.clone(),
        created_at: Some(project.created_at.to_string()),
        updated_at: Some(project.updated_at.to_string()),
    }
}
