use super::{ProjectStatusHistoryInput, ProjectStatusWriteError, insert_project_status_history};
use crate::project::validate_positive_i64;
use a4n_core::models::project::novel_project;
use a4n_domain::project::ProjectStatus;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait, sea_query::Expr,
};

pub async fn update_project_status_record<C>(
    db: &C,
    project: novel_project::Model,
    to_status: ProjectStatus,
    changed_at: NaiveDateTime,
) -> Result<novel_project::Model, ProjectStatusWriteError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_positive_i64(project.id, "project_id")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let txn = db.begin().await?;
    let mut lock_query = novel_project::Entity::find_by_id(project.id)
        .select_only()
        .column(novel_project::Column::Status);
    if txn.get_database_backend() != DbBackend::Sqlite {
        lock_query = lock_query.lock_exclusive();
    }
    let locked_status = lock_query
        .into_tuple::<String>()
        .one(&txn)
        .await
        .map_err(ProjectStatusWriteError::from)?
        .ok_or(ProjectStatusWriteError::ProjectNotFound {
            project_id: project.id,
        })?;
    if locked_status != project.status {
        return Err(ProjectStatusWriteError::StatusChanged {
            expected: project.status,
            actual: locked_status,
        });
    }
    let updated = update_project_status_record_in(&txn, project, to_status, changed_at).await?;
    txn.commit().await?;
    Ok(updated)
}

pub async fn update_project_status_record_in<C>(
    db: &C,
    project: novel_project::Model,
    to_status: ProjectStatus,
    changed_at: NaiveDateTime,
) -> Result<novel_project::Model, ProjectStatusWriteError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project.id, "project_id")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let from_status = ProjectStatus::parse(&project.status).ok_or_else(|| {
        ProjectStatusWriteError::InvalidArgument(format!("未知项目状态：{}", project.status))
    })?;
    if from_status == to_status {
        return Err(ProjectStatusWriteError::InvalidArgument(format!(
            "项目状态没有变化：{}",
            to_status.as_str()
        )));
    }
    let history_reason = if from_status < to_status {
        "项目状态推进"
    } else {
        "项目状态回退"
    };

    let project_id = project.id;
    let mut update = novel_project::Entity::update_many()
        .col_expr(
            novel_project::Column::Status,
            Expr::value(to_status.as_str()),
        )
        .col_expr(novel_project::Column::UpdatedAt, Expr::value(changed_at))
        .filter(novel_project::Column::Id.eq(project_id));
    if to_status == ProjectStatus::Draft {
        update = update
            .col_expr(
                novel_project::Column::RawBlueprint,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                novel_project::Column::TotalChapters,
                Expr::value(Option::<i32>::None),
            );
    }
    let result = update.exec(db).await?;
    if result.rows_affected != 1 {
        return Err(ProjectStatusWriteError::ProjectNotFound { project_id });
    }
    let updated = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or(ProjectStatusWriteError::ProjectNotFound { project_id })?;

    insert_project_status_history(
        db,
        ProjectStatusHistoryInput {
            project_id: updated.id,
            from_status,
            to_status,
            reason: history_reason.to_string(),
            changed_by: "system".to_string(),
            changed_at,
        },
    )
    .await?;

    Ok(updated)
}
